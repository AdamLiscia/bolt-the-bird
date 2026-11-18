"""The imitation player (Phase 4, P2): the P1 turn models make the decisions people make every turn.

- Main phase, own turn, empty stack: the cast model scores the hand once per turn. The player plays
  the best-scoring land it wants, then casts the wanted cards (score ≥ threshold) it can, highest
  first, and passes when none is left.
- Declaring attackers: the attack model scores the creatures; wanted ones attack.
- Declaring blockers: the block model picks the blockers; each blocks the attacker it can kill and
  survive if there is one, else the one it survives, else the first offered.
- Everything else (targets, mulligans, choices mid-resolution, the opponent's turn) is the
  rules-based player's move.
The replays don't record targets or instant-speed play, so those stay with the existing pilots.
"""

import torch

from bolt.turn_model import ZONES

MAIN = ("Main1", "Main2")


def _turn_of_active(turn: int, active: int, first: int) -> int:
    """The active player's own turn count (the replays count each player's turns separately)."""
    return max(1, (turn + 1) // 2 if active == first else turn // 2)


def zones_of(obs: dict, me: int, first: int, attackers: list[int] | None = None, with_ids: bool = False):
    """An observation from `me`'s side as turn-model input: zones of card number + 1, and the
    numbers token (the active player's turn count, life totals, opponent's hand size, on the play)."""
    field = obs["battlefield"]
    by_id = {e["id"]: e for e in field}
    hand = obs["players"][me]["hand"] or []

    def side(owner_is_me: bool, kind: str):
        out = []
        for e in field:
            if (e["controller"] == me) != owner_is_me:
                continue
            what = "land" if e["land"] else "creature" if e["creature"] else "other"
            if what == kind:
                out.append(e)
        return out

    groups = {
        "hand": [(i, c) for i, c in hand],
        "my_lands": [(e["id"], e["card"]) for e in side(True, "land")],
        "my_creatures": [(e["id"], e["card"]) for e in side(True, "creature")],
        "my_others": [(e["id"], e["card"]) for e in side(True, "other")],
        "opp_lands": [(e["id"], e["card"]) for e in side(False, "land")],
        "opp_creatures": [(e["id"], e["card"]) for e in side(False, "creature")],
        "opp_others": [(e["id"], e["card"]) for e in side(False, "other")],
    }
    if attackers is not None:
        groups["attackers"] = [(a, by_id[a]["card"]) for a in attackers if a in by_id]
    zones = {k: torch.tensor([[c + 1 for _, c in v] or [0]], dtype=torch.long) for k, v in groups.items()}
    opp = 1 - me
    scalars = torch.tensor([[
        _turn_of_active(obs["turn"], obs["active"], first) / 20,
        obs["players"][me]["life"] / 20,
        obs["players"][opp]["life"] / 20,
        obs["players"][opp]["hand_size"] / 7,
        float(me == first),
    ]], dtype=torch.float32)  # fmt: skip
    if with_ids:
        return zones, scalars, {k: [i for i, _ in v] for k, v in groups.items()}, by_id
    return zones, scalars


class ImitationPlayer:
    def __init__(self, cast_model, attack_model, block_model, threshold: float = 0.5):
        self.models = {"cast": cast_model, "attack": attack_model, "block": block_model}
        for m in self.models.values():
            m.eval()
        self.threshold = threshold
        self.plans: dict = {}

    @torch.no_grad()
    def _scores(self, task: str, obs: dict, me: int, first: int, attackers=None) -> tuple[dict, dict]:
        zones, scalars, ids, by_id = zones_of(obs, me, first, attackers=attackers, with_ids=True)
        target = "hand" if task == "cast" else "my_creatures"
        probs = torch.sigmoid(self.models[task](zones, scalars, target))[0].tolist()
        return {i: p for i, p in zip(ids[target], probs, strict=False)}, by_id

    def choose(self, game, me: int, first: int) -> int:
        obs = game.observation(me)
        legal = obs["legal_actions"]
        kind, _ = obs["pending"]
        key = (id(game), obs["turn"])
        try:
            if kind == "Priority" and obs["active"] == me and obs["step"] in MAIN and not obs["stack"]:
                return self._main(game, obs, legal, me, first, key)
            if kind == "DeclareAttackers":
                return self._attack(obs, legal, me, first, key)
            if kind == "DeclareBlockers":
                return self._block(obs, legal, me, first, key)
        except (KeyError, IndexError, ValueError):
            pass  # anything unexpected: fall back to the rules-based move
        return game.baseline()

    def _pick(self, legal, kinds) -> int | None:
        for i, a in enumerate(legal):
            if a[0] in kinds:
                return i
        return None

    def _main(self, game, obs, legal, me, first, key) -> int:
        plan = self.plans.get(("cast", key))
        if plan is None:
            scores, by_id = self._scores("cast", obs, me, first)
            hand_cards = dict(obs["players"][me]["hand"] or [])
            land_ids = {i for i, c in hand_cards.items() if any(a[0] == "PlayLand" and a[1] == i for a in legal)}
            wanted = {i: p for i, p in scores.items() if p >= self.threshold}
            lands = sorted((p, i) for i, p in wanted.items() if i in land_ids)
            for _, i in lands[:-1]:
                wanted.pop(i)  # one land a turn: keep the best-scoring one
            plan = self.plans[("cast", key)] = wanted
        best = None
        for i, a in enumerate(legal):
            castable = (a[0] == "PlayLand" or a[0].startswith("Cast")) and a[1] in plan
            if castable and (best is None or plan[a[1]] > plan[legal[best][1]]):
                best = i
        if best is not None:
            plan.pop(legal[best][1])
            return best
        p = self._pick(legal, ("PassPriority",))
        return p if p is not None else game.baseline()

    def _attack(self, obs, legal, me, first, key) -> int:
        plan = self.plans.get(("attack", key))
        if plan is None:
            scores, _ = self._scores("attack", obs, me, first)
            plan = self.plans[("attack", key)] = {i for i, p in scores.items() if p >= self.threshold}
        for i, a in enumerate(legal):
            if a[0] == "Attack" and a[1] in plan:
                plan.discard(a[1])
                return i
        p = self._pick(legal, ("FinishAttacks",))
        if p is None:
            raise ValueError("no way to finish attacks")
        return p

    def _block(self, obs, legal, me, first, key) -> int:
        plan = self.plans.get(("block", key))
        if plan is None:
            attackers = sorted({a[2] for a in legal if a[0] == "Block"})
            scores, _ = self._scores("block", obs, me, first, attackers=attackers)
            plan = self.plans[("block", key)] = {i for i, p in scores.items() if p >= self.threshold}
        by_id = {e["id"]: e for e in obs["battlefield"]}

        def quality(blocker, attacker):
            b, a = by_id[blocker], by_id[attacker]
            kills = b.get("power", 0) >= a.get("toughness", 0) - a.get("damage", 0)
            survives = b.get("toughness", 0) - b.get("damage", 0) > a.get("power", 0)
            return 2 * survives + kills

        options = [(quality(a[1], a[2]), -i, i, a[1]) for i, a in enumerate(legal) if a[0] == "Block" and a[1] in plan]
        if options:
            _, _, i, blocker = max(options)
            plan.discard(blocker)
            return i
        p = self._pick(legal, ("FinishBlocks",))
        if p is None:
            raise ValueError("no way to finish blocks")
        return p


def play_game(deck_a, deck_b, seed: int, first: int, players, max_turns: int = 200) -> str:
    """One game; players[p] is an ImitationPlayer or None for the rules-based player. Returns
    "p0", "p1", "draw" or "timeout"."""
    import bolt_engine as be

    g = be.Game(deck_a, deck_b, seed=seed, first=first)
    for _ in range(100_000):
        if g.is_over():
            return g.result()
        if g.turn > max_turns:
            return "timeout"
        who = g.to_act()
        p = players[who]
        g.step(p.choose(g, who, first) if p is not None else g.baseline())
    return "timeout"


__all__ = ["ZONES", "ImitationPlayer", "play_game", "zones_of"]
