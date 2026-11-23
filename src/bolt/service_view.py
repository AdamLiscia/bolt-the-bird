"""One seat's view of an engine game in the shape docs/api.md describes (the Bolt service's game state).

The engine's observation already hides the opponent's hand and library; this turns it into names,
stable object ids and readable move labels. No game logic here.
"""

import bolt_engine as be

_NAMES = be.card_names()
COLORS = ("White", "Blue", "Black", "Red", "Green")  # the engine's Mode index for ChooseColor (WUBRG)


def name(card_index: int) -> str:
    return _NAMES[card_index]


def _obj(pair) -> dict:
    i, c = pair[0], pair[1]
    return {"id": i, "name": name(c), "card_id": c}


def _label(kind: str, cards: list, nums: tuple, me: int, values: tuple) -> str:
    c = cards[0] if cards else ""
    c2 = cards[1] if len(cards) > 1 else ""
    a, b, _c = values
    simple = {"Keep": "Keep hand", "Mulligan": "Mulligan", "PassPriority": "Pass", "FinishAttacks": "Done attacking",
              "ResetBlocks": "Reset blocks", "FinishBlocks": "Done blocking", "Yes": "Yes", "No": "No"}  # fmt: skip
    if kind in simple:
        return simple[kind]
    if kind == "TargetPlayer":
        return "Target yourself" if a == me else "Target opponent"
    if kind == "Mode":
        return f"Choose mode {a + 1}"
    if kind == "Amount":
        return f"Choose {a}"
    n = nums[0] if nums else None
    return {
        "Bottom": f"Put {c} on the bottom", "PlayLand": f"Play {c}", "Cast": f"Cast {c}",
        "CastKicked": f"Cast {c} (kicked)", "CastWith": f"Cast {c} using {c2}",
        "CastFlashback": f"Cast {c} with flashback", "CastModal": f"Cast {c} (mode {(n or 0) + 1})",
        "CastAlternative": f"Cast {c} (alternative cost)", "CastX": f"Cast {c} with X = {n}",
        "CastFromGraveyard": f"Cast {c} from your graveyard",
        "CastFromGraveyardAs": f"Cast {c} from your graveyard (option {(n or 0) + 1})",
        "Activate": f"Activate {c} (ability {(n or 0) + 1})",
        "ActivateWith": f"Activate {c} (ability {(n or 0) + 1}), using {cards[-1] if len(cards) > 1 else ''}",
        "ActivateX": f"Activate {c} with X = {(b % 256) if b >= 0 else 0}",
        "Loyalty": f"{c}: loyalty ability {(n or 0) + 1}", "Attack": f"Attack with {c}",
        "AttackPlaneswalker": f"Attack {c2} with {c}", "Block": f"Block {c2} with {c}",
        "Target": f"Target {c}", "Pick": f"Choose {c}", "Discard": f"Discard {c}",
    }.get(kind, kind)  # fmt: skip


def _oracle() -> dict:
    import json
    from pathlib import Path

    path = Path(__file__).resolve().parents[2] / "assets" / "FDN_oracle.json"
    try:
        return {c["name"]: c.get("oracle_text") or "" for c in json.loads(path.read_text())}
    except (OSError, ValueError):
        return {}


ORACLE = _oracle()


def _ref(pair) -> dict:
    """An engine (object id, card number) pair as {id, name, card_id}."""
    return {"id": pair[0], "name": name(pair[1]), "card_id": pair[1]}


def _subject(dec: dict) -> tuple[int | None, str]:
    """The card a decision is about (the one being looked at, tucked or protected), else ""."""
    if dec.get("cards"):
        c = dec["cards"][0]
        return c[0], name(c[1])
    if dec.get("card"):
        return dec["card"][0], name(dec["card"][1])
    return None, ""


def _source_name(dec: dict) -> str:
    if dec.get("source"):
        return name(dec["source"][1])
    return name(dec["card"][1]) if dec.get("card") else ""


def _decision_label(dec: dict, kind: str, value: int, card_name: str) -> str | None:
    """A readable label for a move inside a resolving decision, or None to keep the generic one."""
    d = dec.get("kind")
    src = _source_name(dec)
    _, subject = _subject(dec)
    if kind == "Mode":
        if d == "ChooseColor" and 0 <= value < 5:
            return f"{src}: choose {COLORS[value]}"
        if d == "ChooseCreatureType" and 0 <= value < len(dec.get("options", [])):
            return f"{src}: choose {dec['options'][value]}"
        return None
    if kind == "Pick":
        verb = {"SacrificeCreatures": "Sacrifice", "SacrificeAnother": "Sacrifice", "Search": "Take",
                "LookPick": "Take", "KeepOfType": "Keep", "GenesisPick": "Put onto the battlefield:",
                "PickDiscard": "Make them discard"}.get(d)  # fmt: skip
        return f"{verb} {card_name}" if verb else None
    yes = kind == "Yes"
    labels = {
        "Surveil": (f"Put {subject} into your graveyard", f"Keep {subject} on top"),
        "Tuck": (f"Put {subject} on top of your library", f"Put {subject} on the bottom"),
        "May": (f"{src}: yes", f"{src}: no"),
        "MayPay": (f"Pay {dec.get('cost', '')} ({src})", "Don't pay"),
        "PayWard": (f"Pay ward {dec.get('cost', '')} for {subject}", f"Don't pay ({subject} is countered)"),
        "PayWardLife": (f"Pay {dec.get('life', 0)} life for ward ({subject})", f"Don't pay ({subject} is countered)"),
        "SplitPiles": (f"Put {subject} in pile A", f"Put {subject} in pile B"),
        "ChoosePile": ("Choose pile A", "Choose pile B"),
    }
    if d in labels:
        return labels[d][0 if yes else 1]
    if not yes:
        return {"Discard": "Don't discard", "LoseOrDiscard": f"Lose {dec.get('life', 0)} life instead",
                "SacrificeAnother": "Don't sacrifice", "Search": "Find nothing", "LookPick": "Take nothing",
                "GenesisPick": "Done choosing"}.get(d)  # fmt: skip
    return None


def actions(obs: dict, me: int) -> list[dict]:
    dec = obs.get("decision") or {}
    out = []
    for k, ((kind, a, b, c), cards) in enumerate(zip(obs["legal_actions"], obs["legal_cards"], strict=True)):
        values = (a, b, c)
        objects = [values[s] for s in range(3) if cards[s] >= 0]
        names = [name(cards[s]) for s in range(3) if cards[s] >= 0]
        nums = tuple(values[s] for s in range(3) if cards[s] < 0 and values[s] >= 0)
        if kind in ("Mode", "Amount", "TargetPlayer"):
            nums = ()
        label = _label(kind, names, nums, me, values)
        if dec and kind in ("Mode", "Pick", "Yes", "No"):
            label = _decision_label(dec, kind, a, names[0] if names else "") or label
            if kind in ("Yes", "No") and not objects:
                oid, subject = _subject(dec)  # the card the yes/no is about (surveil, tuck, ward)
                if oid is not None and dec.get("kind") in ("Surveil", "Tuck", "PayWard", "PayWardLife", "SplitPiles"):
                    objects, names = [oid], [subject]
        act = {"index": k, "kind": kind, "label": label, "objects": objects, "cards": names}
        if kind == "TargetPlayer":
            act["player"] = a
        elif kind in ("Mode", "Amount"):
            act["number"] = a
        elif kind == "ActivateX":
            act["number"] = b % 256
        elif nums:
            act["number"] = nums[0]
        out.append(act)
    return out


def _pending(kind: str, who: int, dec: dict | None) -> dict:
    """pending, plus what a "Decide" is about: choice (May, Surveil, Tuck, ChooseColor, MayPay, ...),
    source (the asking card) with its rules text, the card or cards involved (looked-at cards only
    reach the decider's view), and any options, cost, life or count."""
    out = {"kind": kind, "player": who}
    if not dec:
        return out
    out["choice"] = dec.get("kind")
    if dec.get("source"):
        out["source"] = _ref(dec["source"])
        out["source_text"] = ORACLE.get(out["source"]["name"], "")
    if dec.get("card"):
        out["card"] = _ref(dec["card"])
    if dec.get("cards"):
        out["cards"] = [_ref(c) for c in dec["cards"]]
    for k in ("a", "b"):
        if k in dec:
            out[f"pile_{k}"] = [_ref(c) for c in dec[k]]
    for k in ("options", "cost", "life", "left", "optional", "x", "what"):
        if k in dec:
            out[k] = dec[k]
    return out


def state(game, seat: int, version: int, game_id: str) -> dict:
    obs = game.observation(seat)
    over = game.is_over()
    players = []
    for p, pl in enumerate(obs["players"]):
        players.append({
            "seat": p, "life": pl["life"], "hand_size": pl["hand_size"], "library": pl["library"],
            "lands_played": pl["lands_played"], "pool": pl["pool"],
            "hand": None if pl["hand"] is None else [_obj(x) for x in pl["hand"]],
            "graveyard": [_obj(x) for x in pl["graveyard"]],
        })  # fmt: skip
    battlefield = []
    for e in obs["battlefield"]:
        battlefield.append({
            "id": e["id"], "name": name(e["card"]), "card_id": e["card"], "controller": e["controller"],
            "owner": e["owner"],
            "tapped": e["tapped"], "creature": e["creature"], "land": e["land"], "power": e.get("power"),
            "toughness": e.get("toughness"), "damage": e["damage"], "counters": e["counters"],
            "loyalty": e["loyalty"], "attached_to": e["attached_to"], "entered_turn": e["entered_turn"],
            "entered_this_turn": e["entered_turn"] == obs["turn"],
        })  # fmt: skip
    combat = obs.get("combat")
    result = None
    if over:
        r = game.result()
        result = {"winner": 0 if r == "p0" else 1 if r == "p1" else None,
                  "outcome": "win" if r in ("p0", "p1") else "draw"}  # fmt: skip
    kind, who = obs["pending"]
    return {
        "game_id": game_id, "version": version, "seat": seat, "turn": obs["turn"], "step": obs["step"],
        "active": obs["active"], "to_act": None if over else game.to_act(),
        "pending": _pending(kind, who, obs.get("decision")), "over": over, "result": result,
        "view": {
            "players": players, "battlefield": battlefield,
            "stack": [{"id": i, "name": name(c), "card_id": c, "controller": ctl} for i, c, ctl in obs["stack"]],
            "exile": [_obj(x) for x in obs["exile"]],
            "combat": None if not combat else {"attackers": list(combat["attackers"]),
                                               "blocks": [list(x) for x in combat["blocks"]]},
        },
        "legal_actions": [] if seat != game.to_act() or over else actions(obs, seat),
    }  # fmt: skip
