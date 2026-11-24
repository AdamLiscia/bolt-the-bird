"""How often a pilot aims pumps and auras at the opponent's creatures, or removal at its own, when it
had a choice of both (2026-11-23: the learned player couldn't see the spell it targeted for).

    python scripts/target_check.py network 200 [--harm] [--ckpt PATH]
"""
import json, re, sys, collections, torch, bolt_engine as be  # noqa: E401
from bolt.play_net import PlayNet
from bolt.play_encoding import encode, collate
torch.set_num_threads(2); torch.set_grad_enabled(False)
oracle = {c["name"]: c.get("oracle_text") or "" for c in json.load(open("assets/FDN_oracle.json"))}
import os
HARM = "--harm" in sys.argv
harm = {n for n, t in oracle.items() if re.search(r"[Dd]estroy target creature|deals \d+ damage to target creature|[Ee]xile target creature|[Tt]arget creature gets -\d+/-\d+", t)}
good = {n for n, t in oracle.items() if re.search(r"[Tt]arget creature( you control)? gets \+\d+/\+\d+|[Tt]arget creature( you control)? gains|Enchanted creature gets \+\d+/\+\d+|\+1/\+1 counters? on target creature", t)
        and not re.search(r"gets -|gets \+\d+/-", t)}
names = be.card_names()
ckpt = sys.argv[sys.argv.index("--ckpt") + 1] if "--ckpt" in sys.argv else "checkpoints/play/play_v2_ppo3_i850.pt"
c = torch.load(ckpt, map_location="cpu", weights_only=False)
net = PlayNet(**c["config"]); net.load_state_dict(c["model"]); net.eval()
lines = open("data/m7/decks.txt").read().split("\n"); D = [list(map(int, x.split())) for x in lines[512:1024]]
def pick(g, kind, seed):
    if kind == "rules": return g.baseline()
    if kind == "smart": return g.smart(1, 2, seed)
    if kind == "deep": return g.smart(4, 6, seed)
    who = g.to_act(); return int(net(collate([encode(g.observation(who), who)]))[0][0].argmax())
kind, n_games = sys.argv[1], int(sys.argv[2])
own = opp = 0; ex = collections.Counter()
for k in range(n_games):
    g = be.Game(D[(2*k) % 512], D[(2*k+1) % 512], seed=900+k, first=k % 2); m = 0; casting = {0: None, 1: None}
    while not g.is_over() and g.turn <= 200 and m < 20000:
        legal = g.legal_actions(); who = g.to_act()
        i = 0 if len(legal) == 1 else pick(g, kind, 900*k + m)
        o = g.observation(who)
        if o["pending"][0] == "ChooseTarget" and casting[who] and len(legal) > 1:
            src = casting[who]
            if src in (harm if HARM else good):
                ctrl = {e["id"]: e["controller"] for e in o["battlefield"]}
                targets = [a for a in legal if a[0] == "Target" and a[1] in ctrl]
                if any(ctrl[a[1]] == who for a in targets) and any(ctrl[a[1]] != who for a in targets):
                    a = legal[i]
                    if a[0] == "Target" and a[1] in ctrl:
                        if ctrl[a[1]] == who: own += 1
                        else: opp += 1; ex[src] += 1
        chosen = legal[i]
        if chosen[0].startswith("Cast"):
            casting[who] = names[o["legal_cards"][i][0]]
        elif o["pending"][0] != "ChooseTarget":
            casting[who] = None
        g.step(i); m += 1
print(f"{kind:8s} games {n_games}: pump/aura targets with a real choice: own {own}, opponent's {opp} ({opp/max(1,own+opp):.1%}) | spells: {dict(ex.most_common(5))}")
