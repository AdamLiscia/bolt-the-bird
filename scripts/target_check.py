"""How often a pilot aims pumps and auras at the opponent's creatures, or removal at its own, when it
had a choice of both (2026-11-23: the learned player couldn't see the spell it targeted for).

    python scripts/target_check.py network 200 [--harm] [--ckpt PATH]

Pilots: rules, smart (1 x 2), deep (4 x 6), network. Counts only target choices where both sides had
a legal creature target.
"""

import collections
import json
import re
import sys

import bolt_engine as be
import torch

from bolt.play_encoding import collate, encode
from bolt.play_net import PlayNet

PUMP = re.compile(
    r"[Tt]arget creature( you control)? gets \+\d+/\+\d+|[Tt]arget creature( you control)? gains"
    r"|Enchanted creature gets \+\d+/\+\d+|\+1/\+1 counters? on target creature"
)
WEAKEN = re.compile(r"gets -|gets \+\d+/-")
HARM = re.compile(
    r"[Dd]estroy target creature|deals \d+ damage to target creature|[Ee]xile target creature"
    r"|[Tt]arget creature gets -\d+/-\d+"
)


def spells(harm: bool) -> set[str]:
    with open("assets/FDN_oracle.json") as f:
        oracle = {c["name"]: c.get("oracle_text") or "" for c in json.load(f)}
    if harm:
        return {n for n, t in oracle.items() if HARM.search(t)}
    return {n for n, t in oracle.items() if PUMP.search(t) and not WEAKEN.search(t)}


def main():
    torch.set_num_threads(2)
    torch.set_grad_enabled(False)
    kind, n_games = sys.argv[1], int(sys.argv[2])
    harm = "--harm" in sys.argv
    ckpt = sys.argv[sys.argv.index("--ckpt") + 1] if "--ckpt" in sys.argv else "checkpoints/play/play_v3_ppo4_i170.pt"
    watched = spells(harm)
    names = be.card_names()
    net = None
    if kind == "network":
        c = torch.load(ckpt, map_location="cpu", weights_only=False)
        net = PlayNet(**c["config"])
        net.load_state_dict(c["model"])
        net.eval()
    with open("data/m7/decks.txt") as f:
        decks = [list(map(int, x.split())) for x in f.read().split("\n")[512:1024]]

    def pick(g, seed):
        if kind == "rules":
            return g.baseline()
        if kind == "smart":
            return g.smart(1, 2, seed)
        if kind == "deep":
            return g.smart(4, 6, seed)
        who = g.to_act()
        return int(net(collate([encode(g.observation(who), who)]))[0][0].argmax())

    own = opp = 0
    wrong = collections.Counter()
    for k in range(n_games):
        g = be.Game(decks[(2 * k) % 512], decks[(2 * k + 1) % 512], seed=900 + k, first=k % 2)
        casting = {0: None, 1: None}  # the spell each player is choosing targets for
        moves = 0
        while not g.is_over() and g.turn <= 200 and moves < 20000:
            legal = g.legal_actions()
            who = g.to_act()
            i = 0 if len(legal) == 1 else pick(g, 900 * k + moves)
            o = g.observation(who)
            src = casting[who]
            if o["pending"][0] == "ChooseTarget" and src in watched and len(legal) > 1:
                ctrl = {e["id"]: e["controller"] for e in o["battlefield"]}
                sides = {ctrl[a[1]] == who for a in legal if a[0] == "Target" and a[1] in ctrl}
                a = legal[i]
                if sides == {True, False} and a[0] == "Target" and a[1] in ctrl:
                    if ctrl[a[1]] == who:
                        own += 1
                    else:
                        opp += 1
                        wrong[src] += 1
            if legal[i][0].startswith("Cast"):
                casting[who] = names[o["legal_cards"][i][0]]
            elif o["pending"][0] != "ChooseTarget":
                casting[who] = None
            g.step(i)
            moves += 1
    what = "removal" if harm else "pump/aura"
    print(
        f"{kind:8s} games {n_games}: {what} targets with a real choice: own {own}, opponent's {opp} "
        f"({opp / max(1, own + opp):.1%}) | spells: {dict(wrong.most_common(5))}"
    )


if __name__ == "__main__":
    main()
