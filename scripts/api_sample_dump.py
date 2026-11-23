"""Sample game states in the docs/api.md shape, for building the Nerd Leagues game screen.

    python scripts/api_sample_dump.py          # -> docs/api_samples/fdn_states.json

Seat 0 (the "human") is played by the rules-based player, seat 1 by the trained network. Every seat-0
decision with a real choice is recorded with the move it took, games continue until every pending
kind the engine has (mulligan, attacks, blocks, targets, yes/no, discards, ...) has appeared, capped
per kind so the file stays small.
"""

import collections
import json
import sys
from pathlib import Path

import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import bolt_engine as be  # noqa: E402

from bolt.play_encoding import collate, encode  # noqa: E402
from bolt.play_net import PlayNet  # noqa: E402
from bolt.service_view import state  # noqa: E402

PER_KIND = 12


def main():
    c = torch.load(ROOT / "checkpoints/play/play_v2_ppo3_i850.pt", map_location="cpu", weights_only=False)
    net = PlayNet(**c["config"])
    net.load_state_dict(c["model"])
    net.eval()
    lines = (ROOT / "data/m7/decks.txt").read_text().split("\n")
    decks = [list(map(int, x.split())) for x in lines[512:600]]
    seen = collections.Counter()
    games = []
    for gi in range(40):
        g = be.Game(decks[2 * gi], decks[2 * gi + 1], seed=4242 + gi, first=gi % 2)
        rec, v = [], 0
        while not g.is_over() and g.turn <= 200 and v < 5000:
            who = g.to_act()
            obs = g.observation(who)
            if who == 1:
                if len(obs["legal_actions"]) == 1:
                    i = 0
                else:
                    with torch.no_grad():
                        i = int(net(collate([encode(obs, 1)]))[0][0].argmax())
            else:
                i = g.baseline()
                kind = obs["pending"][0]
                if len(obs["legal_actions"]) > 1 and seen[kind] < PER_KIND:
                    s = state(g, 0, version=v, game_id=f"sample{gi}")
                    s["chosen_index"] = i
                    rec.append(s)
                    seen[kind] += 1
            g.step(i)
            v += 1
        end = state(g, 0, version=v, game_id=f"sample{gi}")
        games.append({"game": gi, "result": end["result"], "states": rec, "final": end})
        kinds_left = {"Mulligan", "Priority", "DeclareAttackers", "DeclareBlockers", "ChooseTarget", "Decide"} - set(
            seen
        )
        if gi >= 3 and not kinds_left:
            break
    out = {"about": "Seat 0's view at its decisions (docs/api.md state shape) + chosen_index; seat 1 = trained network.",
           "pending_kinds": dict(seen), "games": games}  # fmt: skip
    path = ROOT / "docs/api_samples/fdn_states.json"
    path.write_text(json.dumps(out, indent=1))
    n = sum(len(x["states"]) for x in games)
    print(f"{len(games)} games, {n} states, kinds {dict(seen)} -> {path.relative_to(ROOT)} "
          f"({path.stat().st_size // 1024} KB)")  # fmt: skip


if __name__ == "__main__":
    main()
