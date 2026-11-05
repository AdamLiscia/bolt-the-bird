"""Watch the trained bot draft a full table, build its deck, and get judged.

    python scripts/demo.py              # a random table
    python scripts/demo.py --seed 7     # a specific one

Needs no 17lands download: packs come from assets/ (real opening packs from held-out drafts) and
the trained models download from the GitHub release on first run (~16 MB).
"""

import argparse
import urllib.request

import numpy as np
import torch

from bolt.cards import COLORS, CardInfo
from bolt.data import ASSETS, ROOT
from bolt.deckbuild import build_decks
from bolt.judge import DeckJudge
from bolt.models import DraftTransformer
from bolt.sim import PACK, pools_to_counts, run_tables

RELEASE = "https://github.com/AdamLiscia/bolt-the-bird/releases/download/v0.1.0"
MODELS = {"human_model.pt": "the human-imitation draft model", "bot.pt": "the RL-trained bot",
          "judge_b.pt": "judge B (held out from RL training)"}  # fmt: skip
B, DIM, RESET = "\033[1m", "\033[2m", "\033[0m"
PIP = {"W": "\033[97mW", "U": "\033[94mU", "B": "\033[90mB", "R": "\033[91mR", "G": "\033[92mG"}


def checkpoint(name: str):
    path = ROOT / "checkpoints" / name
    if not path.exists():
        path.parent.mkdir(exist_ok=True)
        print(f"{DIM}downloading {MODELS[name]} ...{RESET}")
        urllib.request.urlretrieve(f"{RELEASE}/{name}", path)
    return torch.load(path, map_location="cpu")


def pips(cards: CardInfo, i: int) -> str:
    c = "".join(ch for k, ch in enumerate(COLORS) if cards.colors[i, k])
    return "".join(PIP[ch] for ch in c) + RESET if c else f"{DIM}·{RESET}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=None)
    a = ap.parse_args()
    seed = a.seed if a.seed is not None else int(np.random.default_rng().integers(10_000))
    torch.set_grad_enabled(False)
    cards = CardInfo()
    human, bot = DraftTransformer(cards.n).eval(), DraftTransformer(cards.n).eval()
    human.load_state_dict(checkpoint("human_model.pt"))
    bot.load_state_dict(checkpoint("bot.pt"))
    judge = DeckJudge(cards.n).eval()
    judge.load_state_dict(checkpoint("judge_b.pt"))
    packs = np.load(ASSETS / "FDN_sample_packs.npy")

    print(f"\n{B}Bolt the Bird{RESET} drafts seat 1 of 8. The other 7 seats draft like people. Table #{seed}\n")
    pools, log = run_tables(packs, 1, bot, human, torch.device("cpu"), torch.Generator().manual_seed(seed),
                            seat0_greedy=True)  # fmt: skip
    p_bot = torch.softmax(bot(log["pool"], log["pack"], log["pack_number"], log["pick_number"]), 1)
    p_hum = torch.softmax(human(log["pool"], log["pack"], log["pack_number"], log["pick_number"]), 1)
    for i in range(42):
        rnd, k = divmod(i, PACK)
        if k == 0:
            print(f"{B}Pack {rnd + 1}{RESET}")
        take = int(log["action"][i])
        human_take = int(p_hum[i].argmax())
        note = "" if human_take == take else f"   {DIM}a person would likely take {cards.names[human_take]}{RESET}"
        if k < 5 or human_take != take:
            print(f"  p{k + 1:<2} {pips(cards, take):<4} {cards.names[take]:<34} {p_bot[i, take]:>4.0%}{note}")
        elif k == 5:
            print(f"  {DIM}... picks that match a person's are hidden from here on{RESET}")

    counts = pools_to_counts(pools[:, :2], cards.n)[0]  # seat 0 (bot) and seat 1 (a human-like drafter)
    decks, wr, labels = build_decks(counts, judge, cards, torch.device("cpu"))
    deck = decks[0]
    print(f"\n{B}The deck{RESET}  {labels[0]}")
    for i in np.flatnonzero(deck * ~cards.land):
        print(f"  {int(deck[i])}x {pips(cards, i)} {cards.names[i]}")
    lands = ", ".join(f"{int(deck[i])} {cards.names[i]}" for i in np.flatnonzero(deck * cards.land))
    print(f"  {DIM}{lands}{RESET}")
    print(f"\n{B}Judge B{RESET} (never seen by the bot during training), average pilot:")
    print(f"  Bolt the Bird's deck     {wr[0]:.1%}")
    print(f"  the next seat's deck     {wr[1]:.1%}   {DIM}(drafted like a person, built by the same builder){RESET}")
    print(f"\n{DIM}One table is noisy. The README's +2.0 points is the average over 512 held-out tables.{RESET}\n")


if __name__ == "__main__":
    main()
