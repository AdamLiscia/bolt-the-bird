"""The Commander training decks: every playgroup deck the engine fully supports, as a deck file (bolt.cmd_decks).

    PYTHONPATH=src python scripts/make_cmd_decks.py [--out handoff/cmd/playable_decks.txt]

Reads ~/bolt-handoff/playgroup_all.json (the playgroup's 31) and lowgame_all.json (the idea decks), the same
lists and the same support test as scripts/debug/soak_cmd.py, plus the engine's Commander deck rules
(be.commander_deck_problems; a wrong card count is allowed). Safe to re-run whenever more cards become
playable: the file is rewritten whole (write then rename), in the source files' order, so the same engine
gives the same file. The network's card table is the whole CMD pool already, so new decks need no re-padding.
"""

import argparse
import json
import re
import sys
from pathlib import Path

import bolt_engine as be

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.cmd_decks import format_line  # noqa: E402

HAND = Path.home() / "bolt-handoff"
SOURCES = [("playgroup_all.json", 31), ("lowgame_all.json", None)]


SIZE = re.compile(r"^\d+ cards with the commanders?, not 100$")


def rule_problems(cmd: list[int], deck: list[int], set_name: str) -> list[str]:
    """The deck's Commander rule problems (commanders, singleton, color identity), its size aside: several of
    the playgroup's lists are 99 or 101 cards, and size doesn't matter for training. An engine without the
    check reports none."""
    check = getattr(be, "commander_deck_problems", None)
    if check is None:
        return []
    return [p for p in check(cmd, deck, set=set_name) if not SIZE.match(p)]


def playable(set_name: str = "CMD", hand: Path = HAND) -> tuple[list[tuple[list[int], list[int], str]], list[str]]:
    """(playable decks as (commanders, cards, name), the names of the decks left out and why)."""
    names = be.card_names(set_name)
    supported = be.supported_mask(set_name)
    index: dict[str, int] = {}
    front: dict[str, int] = {}
    for i, n in enumerate(names):
        index.setdefault(n, i)
        front.setdefault(n.split(" // ")[0], i)

    def card_id(n: str) -> int | None:
        return index[n] if n in index else front.get(n.split(" // ")[0])

    out, skipped = [], []
    for fname, cap in SOURCES:
        d = json.loads((hand / fname).read_text())
        for x in d["deck_order"][:cap]:
            cmd = [x["commander"]] + ([x["partner"]] if x.get("partner") else [])
            name = f"{x['owner']}: {cmd[0].split(',')[0]}"
            cards = list(d["decks"][str(x["deck_id"])])
            for c in cmd:  # the list starts with the commander(s): take each out once
                if c in cards:
                    cards.remove(c)
            ids_c = [card_id(c) for c in cmd]
            ids = [card_id(c) for c in cards]
            unknown = [n for n, i in zip(cmd + cards, ids_c + ids, strict=True) if i is None]
            if unknown:
                skipped.append(f"{name}: {len(unknown)} cards not in {set_name} ({unknown[0]}, ...)")
                continue
            missing = sorted({names[i] for i in ids_c + ids if not (i < len(supported) and supported[i])})
            if missing:
                skipped.append(f"{name}: {len(missing)} unsupported ({', '.join(missing[:3])}, ...)")
                continue
            problems = rule_problems(ids_c, ids, set_name)
            if problems:
                skipped.append(f"{name}: {'; '.join(problems)[:120]}")
                continue
            out.append((ids_c, ids, name))
    return out, skipped


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="CMD")
    ap.add_argument("--out", default=str(ROOT / "handoff" / "cmd" / "playable_decks.txt"))
    a = ap.parse_args()
    decks, skipped = playable(a.set)
    if not decks:
        sys.exit("no playable decks: the deck file is left as it was")
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    tmp = out.with_suffix(".tmp")
    tmp.write_text("".join(format_line(c, d, n) + "\n" for c, d, n in decks))
    tmp.replace(out)
    print(f"{len(decks)} playable decks -> {out}")
    for n in [n for *_, n in decks]:
        print("  +", n)
    print(f"{len(skipped)} left out:")
    for s in skipped:
        print("  -", s)


if __name__ == "__main__":
    main()
