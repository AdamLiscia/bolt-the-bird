"""An engine position and its legal moves as network input (Phase 4, P3a; docs/play/P3_DESIGN.md).

Objects: every card the player can see, as a token (card, zone, numbers): their own hand, both
battlefields, both graveyards, exile and the stack. The opponent's hand is a count only. One vector
of game numbers (turn, step, the pending decision, life totals, hand and library sizes, ...).
Moves: one token per legal move, with its kind, pointers to the objects it names (indices into the
object list, -1 when the object isn't visible or the slot is a number), the cards it names (so a
library search still says which card), its numbers, and which player it targets.
"""

import torch

ZONE = {"my_hand": 0, "my_field": 1, "opp_field": 2, "my_grave": 3, "opp_grave": 4, "exile": 5,
        "my_stack": 6, "opp_stack": 7}  # fmt: skip
FEATS = ["tapped", "creature", "land", "power", "toughness", "damage", "counters", "loyalty", "attached",
         "sick", "attacking", "blocking", "blocked"]  # fmt: skip
STEPS = ["PreGame", "Untap", "Upkeep", "Draw", "Main1", "BeginCombat", "DeclareAttackers", "DeclareBlockers",
         "FirstStrikeDamage", "CombatDamage", "EndCombat", "Main2", "End", "Cleanup"]  # fmt: skip
PENDING = ["Mulligan", "Bottom", "Priority", "DeclareAttackers", "DeclareBlockers", "Discard", "ChooseTarget",
           "Decide", "Divide", "RemoveCounters", "FreeCast", "GameOver"]  # fmt: skip
KINDS = ["Keep", "Mulligan", "Bottom", "PassPriority", "PlayLand", "Cast", "Activate", "ActivateWith", "CastKicked",
         "CastWith", "CastFlashback", "CastModal", "CastAlternative", "CastX", "AttackPlaneswalker", "Loyalty",
         "ActivateX", "TargetPlayer", "Target", "Yes", "No", "Pick", "Mode", "Attack", "FinishAttacks", "Block",
         "ResetBlocks", "FinishBlocks", "Discard", "Amount", "CastFromGraveyard", "CastFromGraveyardAs"]  # fmt: skip
GLOBAL_NAMES = (["turn", "my_turn"] + [f"step_{s}" for s in STEPS] + [f"pending_{p}" for p in PENDING]
                + ["my_life", "opp_life", "my_hand", "opp_hand", "my_library", "opp_library", "lands_played",
                   "pool", "stack"])  # fmt: skip


def encode(obs: dict, me: int) -> dict:
    """One decision point from `me`'s side. Lists, not tensors; `collate` batches them."""
    opp = 1 - me
    combat = obs.get("combat") or {"attackers": [], "blocks": []}
    attacking = set(combat["attackers"])
    blocking = {b for b, _ in combat["blocks"]}
    blocked = {a for _, a in combat["blocks"]}
    obj_id, card, zone, feats = [], [], [], []

    def add(i, c, z, f=None):
        obj_id.append(i)
        card.append(c + 1)
        zone.append(ZONE[z])
        feats.append(f or [0.0] * len(FEATS))

    for i, c in obs["players"][me]["hand"] or []:
        add(i, c, "my_hand")
    for e in obs["battlefield"]:
        f = [
            float(e["tapped"]), float(e["creature"]), float(e["land"]), e.get("power", 0) / 10,
            e.get("toughness", 0) / 10, e["damage"] / 10, e["counters"] / 5, e["loyalty"] / 10,
            float(e["attached_to"] is not None),
            float(e["creature"] and e["entered_turn"] == obs["turn"]),
            float(e["id"] in attacking), float(e["id"] in blocking), float(e["id"] in blocked),
        ]  # fmt: skip
        add(e["id"], e["card"], "my_field" if e["controller"] == me else "opp_field", f)
    for i, c in obs["players"][me]["graveyard"]:
        add(i, c, "my_grave")
    for i, c in obs["players"][opp]["graveyard"]:
        add(i, c, "opp_grave")
    for i, c in obs["exile"]:
        add(i, c, "exile")
    for i, c, ctrl in obs["stack"]:
        add(i, c, "my_stack" if ctrl == me else "opp_stack")
    # The spell or ability whose targets are being chosen: on its way to the stack (the engine lists it
    # only once its targets are set). Without it a target choice can't tell a pump from removal.
    ch = obs.get("choosing")
    if ch and ch["id"] not in {x for x in obj_id}:
        add(ch["id"], ch["card"], "my_stack" if ch["controller"] == me else "opp_stack")

    kind, _ = obs["pending"]
    p = obs["players"]
    g = [obs["turn"] / 30, float(obs["active"] == me)]
    g += [float(obs["step"] == s) for s in STEPS] + [float(kind == k) for k in PENDING]
    g += [p[me]["life"] / 20, p[opp]["life"] / 20, p[me]["hand_size"] / 10, p[opp]["hand_size"] / 10,
          p[me]["library"] / 40, p[opp]["library"] / 40, float(p[me]["lands_played"]), p[me]["pool"] / 10,
          len(obs["stack"]) / 5]  # fmt: skip

    where = {i: k for k, i in enumerate(obj_id)}
    act_kind, act_ptr, act_card, act_num, act_player = [], [], [], [], []
    for (k, a, b, c), cards in zip(obs["legal_actions"], obs["legal_cards"], strict=True):
        act_kind.append(KINDS.index(k) if k in KINDS else len(KINDS))
        values = (a, b, c)
        act_ptr.append([where.get(v, -1) if cards[s] >= 0 else -1 for s, v in enumerate(values)])
        act_card.append([cards[s] + 1 if cards[s] >= 0 else 0 for s in range(3)])
        nums = [v for s, v in enumerate(values) if cards[s] < 0 and v >= 0 and k != "TargetPlayer"]
        act_num.append([(nums[0] if nums else 0) / 10, (nums[1] if len(nums) > 1 else 0) / 10])
        act_player.append((1 if a == me else 2) if k == "TargetPlayer" else 0)
    return {
        "obj_id": obj_id, "card": card, "zone": zone, "feats": feats, "globals": g, "global_names": GLOBAL_NAMES,
        "act_kind": act_kind, "act_ptr": act_ptr, "act_card": act_card, "act_num": act_num, "act_player": act_player,
    }  # fmt: skip


def collate(batch: list[dict]) -> dict:
    """Pad a list of encodings into tensors. Masks are True where a slot is padding."""
    b = len(batch)
    t = max(1, max(len(e["card"]) for e in batch))
    a = max(len(e["act_kind"]) for e in batch)
    out = {
        "card": torch.zeros(b, t, dtype=torch.long),
        "zone": torch.zeros(b, t, dtype=torch.long),
        "feats": torch.zeros(b, t, len(FEATS)),
        "obj_pad": torch.ones(b, t, dtype=torch.bool),
        "globals": torch.tensor([e["globals"] for e in batch], dtype=torch.float32),
        "act_kind": torch.zeros(b, a, dtype=torch.long),
        "act_ptr": torch.full((b, a, 3), -1, dtype=torch.long),
        "act_card": torch.zeros(b, a, 3, dtype=torch.long),
        "act_num": torch.zeros(b, a, 2),
        "act_player": torch.zeros(b, a, dtype=torch.long),
        "act_pad": torch.ones(b, a, dtype=torch.bool),
    }
    for i, e in enumerate(batch):
        n, m = len(e["card"]), len(e["act_kind"])
        if n:
            out["card"][i, :n] = torch.tensor(e["card"])
            out["zone"][i, :n] = torch.tensor(e["zone"])
            out["feats"][i, :n] = torch.tensor(e["feats"])
            out["obj_pad"][i, :n] = False
        out["act_kind"][i, :m] = torch.tensor(e["act_kind"])
        out["act_ptr"][i, :m] = torch.tensor(e["act_ptr"])
        out["act_card"][i, :m] = torch.tensor(e["act_card"])
        out["act_num"][i, :m] = torch.tensor(e["act_num"])
        out["act_player"][i, :m] = torch.tensor(e["act_player"])
        out["act_pad"][i, :m] = False
    return out
