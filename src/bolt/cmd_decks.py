"""Commander deck files: one deck a line, its commander card numbers, then its other cards (BOLT_PLAY_SET's
numbers, CMD by default), then the deck's name:

    2995 | 12 12 40 77 ... # Owner: Commander

Two commanders (partners) are "a,b". scripts/make_cmd_decks.py writes data/cmd/playable_decks.txt from the
playgroup's deck lists, keeping only decks the engine fully supports; `read` loads one.
"""

from pathlib import Path

Deck = tuple[list[int], list[int]]  # (commanders, the other cards)


def format_line(cmd: list[int], cards: list[int], name: str = "") -> str:
    line = ",".join(map(str, cmd)) + " | " + " ".join(map(str, cards))
    return line + (f" # {name}" if name else "")


def parse_line(line: str) -> tuple[Deck, str]:
    body, _, name = line.partition("#")
    cmd, sep, cards = body.partition("|")
    if not sep:
        raise ValueError(f"not a Commander deck line (no '|'): {line[:60]!r}")
    return ([int(x) for x in cmd.split(",")], [int(x) for x in cards.split()]), name.strip()


def read(path) -> list[Deck]:
    """The decks in a deck file, in order (blank lines and lines starting with '#' skipped)."""
    out = []
    for line in Path(path).read_text().split("\n"):
        if line.strip() and not line.lstrip().startswith("#"):
            out.append(parse_line(line)[0])
    return out


def names(path) -> list[str]:
    return [
        parse_line(x)[1] for x in Path(path).read_text().split("\n") if x.strip() and not x.lstrip().startswith("#")
    ]
