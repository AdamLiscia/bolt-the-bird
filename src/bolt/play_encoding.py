"""An engine position and its legal moves as network input (Phase 4, P3a; docs/play/P3_DESIGN.md).

Objects: every card the player can see, as a token (card, zone, numbers): their own hand, both
battlefields, both graveyards, exile and the stack. The opponent's hand is a count only. One vector
of game numbers (turn, step, the pending decision, life totals, hand and library sizes, ...).
Moves: one token per legal move, with its kind, pointers to the objects it names (indices into the
object list, -1 when the object isn't visible or the slot is a number), the cards it names (so a
library search still says which card), its numbers, and which player it targets.
"""

import functools
import json
import os
from pathlib import Path

import torch

# Everything below is append-only: a trained network's rows for zones, features, globals and kinds keep
# their meaning, and bolt.play_net.load_checkpoint zero-pads the inputs it never saw (2026-11-27: Commander).
ZONE = {"my_hand": 0, "my_field": 1, "opp_field": 2, "my_grave": 3, "opp_grave": 4, "exile": 5,
        "my_stack": 6, "opp_stack": 7, "my_command": 8, "opp_command": 9}  # fmt: skip
# The per-object numbers the engine reports (the first 13), then Commander's (is it a commander, its tax),
# then the card's printed facts (STATIC, from assets/<SET>_oracle.json), so an unseen card still reads as
# a 3-mana 2/2 flier rather than as nothing.
DYNAMIC = ["tapped", "creature", "land", "power", "toughness", "damage", "counters", "loyalty", "attached",
           "sick", "attacking", "blocking", "blocked", "commander", "commander_tax"]  # fmt: skip
COLORS = "WUBRG"
TYPES = ["Creature", "Land", "Artifact", "Enchantment", "Planeswalker", "Instant", "Sorcery", "Legendary"]
KEYWORDS = ["Flying", "Trample", "Deathtouch", "Lifelink", "Haste", "Vigilance", "First strike", "Menace",
            "Hexproof", "Indestructible"]  # fmt: skip
STATIC = (["mana_value"] + [f"color_{c}" for c in COLORS] + [f"type_{t.lower()}" for t in TYPES]
          + ["base_power", "base_toughness"] + [f"kw_{k.lower().replace(' ', '_')}" for k in KEYWORDS]
          + ["printed"])  # fmt: skip
FEATS = DYNAMIC + STATIC
ROOT = Path(__file__).resolve().parents[2]
STEPS = ["PreGame", "Untap", "Upkeep", "Draw", "Main1", "BeginCombat", "DeclareAttackers", "DeclareBlockers",
         "FirstStrikeDamage", "CombatDamage", "EndCombat", "Main2", "End", "Cleanup"]  # fmt: skip
PENDING = ["Mulligan", "Bottom", "Priority", "DeclareAttackers", "DeclareBlockers", "Discard", "ChooseTarget",
           "Decide", "Divide", "RemoveCounters", "FreeCast", "GameOver"]  # fmt: skip
KINDS = [
    "Keep",
    "Mulligan",
    "Bottom",
    "PassPriority",
    "PlayLand",
    "Cast",
    "Activate",
    "ActivateWith",
    "CastKicked",
    "CastWith",
    "CastFlashback",
    "CastModal",
    "CastAlternative",
    "CastX",
    "AttackPlaneswalker",
    "Loyalty",
    "ActivateX",
    "TargetPlayer",
    "Target",
    "Yes",
    "No",
    "Pick",
    "Mode",
    "Attack",
    "FinishAttacks",
    "Block",
    "ResetBlocks",
    "FinishBlocks",
    "Discard",
    "Amount",
    "CastFromGraveyard",
    "CastFromGraveyardAs",
    "CastHarmonize",
    "CastOmen",
]  # fmt: skip (Tarkir kinds appended 2026-11-25; load_checkpoint pads)
GLOBAL_NAMES = (["turn", "my_turn"] + [f"step_{s}" for s in STEPS] + [f"pending_{p}" for p in PENDING]
                + ["my_life", "opp_life", "my_hand", "opp_hand", "my_library", "opp_library", "lands_played",
                   "pool", "stack"]
                + ["is_commander_game", "cmd_damage_taken", "cmd_damage_dealt", "my_cmd_tax", "opp_cmd_tax"])  # fmt: skip


def _num(x) -> float:
    try:
        return float(x)
    except (TypeError, ValueError):  # "*", "1+*", None
        return 0.0


def static_rows(names: list[str], oracle: list[dict]) -> list[list[float]]:
    """One STATIC row per card number: its printed facts, matched by name (or front face). A name with no
    oracle entry (a token) gets zeros, "printed" included."""
    by_name = {}
    for o in oracle:
        by_name.setdefault(o["name"], o)
        by_name.setdefault(o["name"].split(" // ")[0], o)
    rows = []
    for n in names:
        o = by_name.get(n) or by_name.get(n.split(" // ")[0])
        if o is None:
            rows.append([0.0] * len(STATIC))
            continue
        tl = o.get("type_line", "").split(" // ")[0].split()
        kws = {k.lower() for k in o.get("keywords") or []}
        text = (o.get("oracle_text") or "").lower()
        row = [_num(o.get("cmc")) / 10]
        row += [float(c in (o.get("colors") or [])) for c in COLORS]
        row += [float(t in tl) for t in TYPES]
        row += [_num(o.get("power")) / 10, _num(o.get("toughness")) / 10]
        # Scryfall's keyword list misses some (granted keywords aside); the first line of rules text has them
        first = text.split("\n")[0]
        row += [float(k.lower() in kws or k.lower() in first.split(", ")) for k in KEYWORDS]
        rows.append(row + [1.0])
    return rows


@functools.cache
def card_table(set_name: str) -> tuple:
    """STATIC rows by card number for a set (the engine's card names against its oracle file); empty when
    either is missing, which makes every row zeros."""
    path = ROOT / "assets" / f"{set_name}_oracle.json"
    try:
        import bolt_engine as be

        names = be.card_names() if set_name == "FDN" else be.card_names(set_name)
        oracle = json.loads(path.read_text())
    except (ImportError, OSError, TypeError, ValueError):
        return ()
    return tuple(tuple(r) for r in static_rows(names, oracle))


def _static(table, c: int) -> list[float]:
    return list(table[c]) if 0 <= c < len(table) else [0.0] * len(STATIC)


def _taxes(pl: dict) -> dict:
    """Commander id -> tax for one player's view (the observation's commander_tax, or the command zone)."""
    if "commander_tax" in pl:
        return dict(pl["commander_tax"])
    return {i: t for i, _c, t in pl.get("command") or []}


def encode(obs: dict, me: int, set_name: str | None = None) -> dict:
    """One decision point from `me`'s side. Lists, not tensors; `collate` batches them. `set_name` picks the
    printed-facts table (default BOLT_PLAY_SET, else FDN)."""
    opp = 1 - me
    table = card_table((set_name or os.environ.get("BOLT_PLAY_SET") or "FDN").upper())
    p = obs["players"]
    cmd_ids = {i for i, _ in obs.get("commanders") or []}
    tax = {**_taxes(p[0]), **_taxes(p[1])}
    combat = obs.get("combat") or {"attackers": [], "blocks": []}
    attacking = set(combat["attackers"])
    blocking = {b for b, _ in combat["blocks"]}
    blocked = {a for _, a in combat["blocks"]}
    obj_id, card, zone, feats = [], [], [], []

    def add(i, c, z, f=None):
        obj_id.append(i)
        card.append(c + 1)
        zone.append(ZONE[z])
        f = f or [0.0] * 13
        f += [float(i in cmd_ids), tax.get(i, 0) / 10]
        feats.append(f + _static(table, c))

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
    # Commander: each command zone (public). A commander on the stack with targets still to choose is the
    # `choosing` entry above, so it isn't listed twice.
    for who, z in ((me, "my_command"), (opp, "opp_command")):
        for i, c, _tax in p[who].get("command") or []:
            if i not in obj_id:
                add(i, c, z)

    kind, _ = obs["pending"]
    # Life and library scale with the starting life (20 or 40), so a Commander game at 40 reads like a
    # 20-life game at 20; 20-life games encode exactly as before.
    is_cmd = bool(cmd_ids)
    start = obs.get("starting_life") or (40 if is_cmd else 20)
    k = start / 20
    g = [obs["turn"] / 30, float(obs["active"] == me)]
    g += [float(obs["step"] == s) for s in STEPS] + [float(kind == k) for k in PENDING]
    g += [p[me]["life"] / start, p[opp]["life"] / start, p[me]["hand_size"] / 10, p[opp]["hand_size"] / 10,
          p[me]["library"] / (40 * k), p[opp]["library"] / (40 * k), float(p[me]["lands_played"]),
          p[me]["pool"] / 10, len(obs["stack"]) / 5]  # fmt: skip
    # Commander damage: the most from any one commander (21 is lethal), taken by me and dealt by mine.
    taken = max((n for *_, n in p[me].get("commander_damage") or []), default=0)
    dealt = max((n for _i, _c, o, n in p[opp].get("commander_damage") or [] if o == me), default=0)
    owner = dict(obs.get("commanders") or [])
    my_tax = sum(t for i, t in tax.items() if owner.get(i) == me)
    opp_tax = sum(t for i, t in tax.items() if owner.get(i) == opp)
    g += [float(is_cmd), taken / 21, dealt / 21, my_tax / 10, opp_tax / 10]

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
