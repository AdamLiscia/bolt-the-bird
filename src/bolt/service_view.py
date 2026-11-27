"""One seat's view of an engine game in the shape docs/api.md describes (the Bolt service's game state).

The engine's observation already hides the opponent's hand and library; this turns it into names,
stable object ids and readable move labels. No game logic here.
"""

import os

import bolt_engine as be

# One card set per service process (BOLT_SET, default FDN; the Tarkir service runs with TDM).
SET = os.environ.get("BOLT_SET", "FDN").upper()
_NAMES = be.card_names() if SET == "FDN" else be.card_names(SET)
COLORS = ("White", "Blue", "Black", "Red", "Green")  # the engine's Mode index for ChooseColor (WUBRG)
SACRIFICE_FOR_MANA = {"Treasure"}  # tokens whose mana ability sacrifices them


FACE_DOWN = -2  # the engine's card number for a face-down permanent the viewer can't see (manifest dread)


def name(card_index: int) -> str:
    if card_index == FACE_DOWN:
        return "Face-down creature"
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
    if kind == "TapForMana":
        sym = "WUBRGC"[b] if 0 <= b <= 5 else "C"
        verb = "Sacrifice" if c in SACRIFICE_FOR_MANA else "Tap"
        return f"{verb} {c} for {{{sym}}}"
    if kind == "Mode":
        return f"Choose mode {a + 1}"
    if kind == "Amount":
        return f"Choose {a}"
    if kind == "CastOmen" and " // " in c:  # an Omen's or a split card's second half, by its name
        return f"Cast {c.split(' // ', 1)[1]}"
    n = nums[0] if nums else None
    if kind == "CastModal" and n == 1 and " // " in c:  # a modal double-faced card's back face
        return f"Cast {c.split(' // ', 1)[1]}"
    if kind == "CastFromGraveyardAs" and n == 5:  # KARADOR_CAST: Karador's once-each-turn creature
        return f"Cast {c} from your graveyard (Karador)"
    return {
        "Bottom": f"Put {c} on the bottom", "PlayLand": f"Play {c}", "Cast": f"Cast {c}",
        "CastKicked": f"Cast {c} (kicked)", "CastWith": f"Cast {c} using {c2}",
        "CastFlashback": f"Cast {c} with flashback", "CastModal": f"Cast {c} (mode {(n or 0) + 1})",
        "CastAlternative": f"Cast {c} (alternative cost)", "CastX": f"Cast {c} with X = {n}",
        "CastFromGraveyard": f"Cast {c} from your graveyard",
        "CastHarmonize": f"Cast {c} with harmonize, tapping {c2}",
        "CastOmen": f"Cast {c}'s Omen",
        "CastFromGraveyardAs": f"Cast {c} from your graveyard (option {(n or 0) + 1})",
        "Activate": f"Activate {c} (ability {(n or 0) + 1})",
        "ActivateWith": f"Activate {c} (ability {(n or 0) + 1}), using {cards[-1] if len(cards) > 1 else ''}",
        "ActivateX": f"Activate {c} with X = {(b % 256) if b >= 0 else 0}",
        "Loyalty": f"{c}: loyalty ability {(n or 0) + 1}", "Attack": f"Attack with {c}",
        "AttackPlaneswalker": f"Attack {c2} with {c}", "Block": f"Block {c2} with {c}",
        "Target": f"Target {c}", "Pick": f"Choose {c}", "Discard": f"Discard {c}",
    }.get(kind, kind)  # fmt: skip


def _oracle_rows() -> list:
    import json
    from pathlib import Path

    path = Path(__file__).resolve().parents[2] / "assets" / f"{SET}_oracle.json"
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return []


_ROWS = _oracle_rows()
ORACLE = {c["name"]: c.get("oracle_text") or "" for c in _ROWS}
BACKS = {c["name"]: c["back"]["name"] for c in _ROWS if c.get("back")}  # double-faced card -> back face name


def back_image(card_name: str) -> str:
    """Scryfall's image of a double-faced card's back face (the by-name image link with face=back)."""
    import urllib.parse

    q = urllib.parse.urlencode({"exact": card_name, "format": "image", "version": "normal", "face": "back"})
    return f"https://api.scryfall.com/cards/named?{q}"


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
    card = name(dec["card"][1]) if dec.get("card") else ""
    if kind == "Amount":
        if d == "ChooseCastX":
            return f"Cast {card} with X = {value}" if card else f"X = {value}"
        if d == "ChooseLoyaltyX":
            return f"{src}: \u2212X with X = {value}" if src else f"X = {value}"
        if d == "PayXDamage":
            return "Don't pay" if value == 0 else f"Pay {{{value}}}: {value} damage" + (f" ({src})" if src else "")
        if d == "PayXDrawX":
            if value == 0:
                return "Don't pay"
            word = "card" if value == 1 else "cards"
            return f"Pay {{{value}}}: draw {value} {word}" + (f" ({src})" if src else "")
        if d == "ChooseDelve":
            word = "card" if value == 1 else "cards"
            return f"Delve: exile {value} {word} from your graveyard" + (f" ({card})" if card else "")
        if d == "ChooseKickCount":
            times = "once" if value == 1 else f"{value} times"
            return f"Kick {card} {times}" if card else f"Kick {times}"
        return None
    if kind == "Mode":
        if d == "ChooseColor" and 0 <= value < 5:
            return f"{src}: choose {COLORS[value]}"
        if d == "ChooseCreatureType" and 0 <= value < len(dec.get("options", [])):
            return f"{src}: choose {dec['options'][value]}"
        if d == "ChooseLandFace" and value in (0, 1):
            faces = (src or "").split(" // ")
            face = faces[value] if len(faces) == 2 else ("its front", "its back")[value]
            return f"Play it as {face}"
        if d == "ProtectionColor" and 0 <= value < 5:
            return f"Protection from {COLORS[value].lower()}"
        if d == "ChooseCardType" and 0 <= value < 7:
            kinds = ("artifacts", "creatures", "enchantments", "instants", "lands", "planeswalkers", "sorceries")
            return f"{src or 'It'}: protection from {kinds[value]}"
        if d == "ProtectionCardType" and 0 <= value < 7:
            kinds = ("artifacts", "creatures", "enchantments", "instants", "lands", "planeswalkers", "sorceries")
            return f"Protection from {kinds[value]}"
        if d == "Riot" and value in (0, 1):
            return f"{card or 'It'}: riot, " + ("a +1/+1 counter" if value == 0 else "haste")
        if d == "TargetToSource":
            return f"Change target {value + 1} to {src or 'this creature'}"
        if d == "ChooseDungeon" and 0 <= value < len(dec.get("options", [])):
            return f"Venture into {dec['options'][value]}"
        if d == "ChooseRoom" and 0 <= value < len(dec.get("options", [])):
            return f"Venture into the room {dec['options'][value]}"
        return None
    if kind == "Pick" and d == "SacrificeAnotherGainDraw":
        return f"Sacrifice {card_name} (gain life and draw equal to its power)"
    if kind == "Pick" and d == "ReturnLandOrSacrifice":
        return f"Return {card_name} to your hand"
    if kind == "No" and d == "ReturnLandOrSacrifice":
        return f"Sacrifice {src} instead" if src else "Sacrifice it instead"
    if kind == "Pick":
        verb = {"SacrificeCreatures": "Sacrifice", "SacrificeAnother": "Sacrifice", "Search": "Take",
                "LookPick": "Take", "KeepOfType": "Keep", "ScryOrder": "Put on top:", "GenesisPick": "Put onto the battlefield:", "PutPicksOntoBattlefield": "Put onto the battlefield:", "LandFromHandTapped": "Put onto the battlefield tapped:",
                "DistributeCounters": "Put a +1/+1 counter on", "ManifestDread": "Manifest", "TurnFaceUp": "Turn face up:",
                "PickVoidCard": "Play free this turn:", "ReturnOwnCreatureOfColor": "Return to its owner's hand:", "CacheGrabPick": "Put into your hand:", "MoveCounterFrom": "Move a +1/+1 counter onto", "ExpropriatePick": "Gain control of", "LandFromHand": "Put onto the battlefield:",
                "PickDiscard": "Make them discard", "EnterAsCopy": f"{card or 'It'} enters as a copy of"}.get(d)  # fmt: skip
        verb = verb or {"CreaturesFromHandUpTo": "Put onto the battlefield:", "PermanentsFromHandUpTo": "Put onto the battlefield:", "ReturnOwnPermanent": "Return", "SearchToGraveyardCounter": "Put into your graveyard:"}.get(d)  # fmt: skip
        verb = verb or {"KeepNonlandOfType": "Keep", "HandToBottom": "Put on the bottom:", "SacrificeAnyNumber": "Sacrifice", "SacrificeCreatureOrPlaneswalkerEach": "Sacrifice", "ReturnAnotherPermanentCard": "Return to your hand:", "BasicsTappedUpTo": "Put onto the battlefield tapped:", "SignalTheClans": "Reveal", "BraidsSacrifice": "Sacrifice", "BraidsOpponent": "Sacrifice"}.get(d)  # fmt: skip
        verb = verb or {"ConniveDiscard": "Discard (connive):"}.get(d)
        verb = verb or {"LandOrCreatureFromHandTapped": "Put onto the battlefield tapped:"}.get(d)
        verb = verb or {"ExiledWithToGraveyard": "Put into its owner's graveyard:"}.get(d)
        verb = verb or {"VowCreature": "Put a vow counter on (sacrifice the rest):", "DiscardOrLoseLife": "Discard"}.get(d)  # fmt: skip
        verb = verb or {"SacrificeOfType": "Sacrifice", "RevealedCreatureOnto": "Put onto the battlefield:"}.get(d)  # fmt: skip
        verb = verb or {"SacrificeAnotherPermanent": "Sacrifice", "TakeMilledCard": "Put into your hand:"}.get(d)
        verb = verb or {"ReturnSharingType": "Return to its owner's hand:", "ExileOwnPermanent": "Exile"}.get(d)
        verb = verb or {"UntapOwnCreature": "Untap"}.get(d)
        verb = verb or {"PutFromOpponentsTop": "Put onto the battlefield under your control:"}.get(d)
        verb = verb or {"ExileUntilYourNextUpkeep": "Exile until your next upkeep:"}.get(d)
        verb = verb or {"CastFromGraveyardThisTurn": "You may cast this turn (then exile it):"}.get(d)
        verb = verb or {"TakeTwoBottomRest": "Put into your hand:"}.get(d)
        verb = verb or {"SacrificeDiscardOrLoseLife": "Sacrifice (or discard, from your hand):"}.get(d)
        verb = verb or {"ReanimateTotalManaValue": "Put onto the battlefield under your control:"}.get(d)
        verb = verb or {"SacrificeNontokenCreatureEach": "Sacrifice"}.get(d)
        verb = verb or {"SacrificeNontokenCreatureOrPlaneswalker": "Sacrifice"}.get(d)
        verb = verb or {"SacrificeNontokenArtifact": "Sacrifice", "ExileThreeForFood": "Exile (three for a Food):"}.get(d)  # fmt: skip
        verb = verb or {"SylvanPick": "Choose (pay 4 life or put it back):"}.get(d)
        verb = verb or {"CrewTap": "Also tap (to crew or saddle):"}.get(d)
        verb = verb or {"Dredge": "Dredge instead of drawing:"}.get(d)
        if d == "IterationPick":
            verb = "Put into your hand:" if dec.get("to_hand") else "Put on the bottom:"
        verb = verb or {"ChooseRingBearer": "Choose as your Ring-bearer:"}.get(d)
        verb = verb or {"AttachAura": "Attach"}.get(d)
        verb = verb or {"AuraEntersAttachedTo": "Attach the Aura to"}.get(d)
        verb = verb or {"DiscardThenIfInstantOrSorcery": "Discard"}.get(d)
        verb = verb or {"MoveAuraTo": "Attach the Aura to"}.get(d)
        verb = verb or {"CreatureFromHandAttacking": "Put onto the battlefield attacking:"}.get(d)
        verb = verb or {"PatronOfChaosSacrifice": "Sacrifice (one of two):"}.get(d)
        verb = verb or {"ExileFromHandUntilSourceLeaves": "Exile until it leaves:"}.get(d)
        if d == "LoseUnless":
            verb = "Sacrifice" if dec.get("sacrifice") else "Discard"
        return f"{verb} {card_name}" if verb else None
    yes = kind == "Yes"
    labels = {
        "Surveil": (f"Put {subject} into your graveyard", f"Keep {subject} on top"),
        "Scry": (f"Put {subject} on the bottom", f"Keep {subject} on top"),
        "Endure": (
            f"{src}: put {dec.get('x', 0)} +1/+1 counters on it",
            f"{src}: create a {dec.get('x', 0)}/{dec.get('x', 0)} Spirit token",
        ),
        "Tuck": (f"Put {subject} on top of your library", f"Put {subject} on the bottom"),
        "May": (f"{src}: yes", f"{src}: no"),
        "MayPay": (f"Pay {dec.get('cost', '')} ({src})", "Don't pay"),
        "PayOrLoseGame": (
            f"Pay {dec.get('cost', '')} ({src})" if src else f"Pay {dec.get('cost', '')}",
            "Don't pay (you lose the game)",
        ),
        "PayWard": (f"Pay ward {dec.get('cost', '')} for {subject}", f"Don't pay ({subject} is countered)"),
        "PayWardLife": (f"Pay {dec.get('life', 0)} life for ward ({subject})", f"Don't pay ({subject} is countered)"),
        "PayOrCountered": (f"Pay {dec.get('cost', '')} ({subject} resolves)", f"Don't pay ({subject} is countered)"),
        "SplitPiles": (f"Put {subject} in pile A", f"Put {subject} in pile B"),
        "ShockLand": (f"Pay 2 life: {subject} enters untapped", f"{subject} enters tapped"),
        "CommanderToCommandZone": (f"Move {subject} to the command zone", f"Leave {subject} there"),
        "ChoosePile": ("Choose pile A", "Choose pile B"),
        "PayToStop": (f"Pay {dec.get('cost', '')}", "Don't pay"),
        "MayDrawGainOne": ("Draw a card and gain 1 life", "Don't draw"),
        "SylvanDraw": ("Draw two additional cards", "Don't draw"),
        "CrewTap": ("Tap the chosen creatures", "Don't tap them"),
        "Dredge": ("Dredge", "Draw a card"),
        "MirrormindCopies": ("Make copies of the equipped creature instead", "Make the tokens"),
        "SylvanPayOrTop": (f"Pay 4 life to keep {subject or 'it'}", f"Put {subject or 'it'} on top of your library"),
        "ExpropriateVote": ("Vote time (an extra turn)", "Vote money (a permanent)"),
        "PayUpkeep": (f"Pay the upkeep {dec.get('cost', '')}", f"Don't pay (sacrifice {subject or src})"),
        "MayShuffle": ("Shuffle your library", "Don't shuffle"),
        "TaintedPact": ("Put it into your hand", "Exile the next card"),
        "MayPayLife": (
            f"Pay {dec.get('life', 0)} life ({src})" if src else f"Pay {dec.get('life', 0)} life",
            "Don't pay",
        ),
        "MayPayManaAndLife": (f"Pay {dec.get('cost', '')} and {dec.get('life', 0)} life", "Don't pay"),
        "Fabricate": (
            f"{src or 'Fabricate'}: put {dec.get('x', 0)} +1/+1 counters on it",
            f"{src or 'Fabricate'}: create {dec.get('x', 0)} 1/1 Servo tokens",
        ),
        "LetDrawOrMill": (
            "Let them draw a card",
            f"Make them mill {dec.get('x', 0)} (you lose life equal to the total mana value)",
        ),
    }
    if d in ("CreaturesFromHandUpTo", "PermanentsFromHandUpTo", "SearchToGraveyardCounter") and not yes:
        return "Find nothing" if d == "SearchToGraveyardCounter" else "Done choosing"
    if d == "PutPicksOntoBattlefield" and not yes:
        return "Done choosing"
    if d == "PutFromOpponentsTop" and not yes:
        return "Put none onto the battlefield"
    if d == "ExileUntilYourNextUpkeep" and not yes:
        return "Done exiling"
    if d == "AttachAura" and not yes:
        return "Done attaching Auras"
    if d == "PatronOfChaosSacrifice" and not yes:
        return "Don't sacrifice (they draw two cards)"
    if d == "ExileFromHandUntilSourceLeaves" and not yes:
        return "Exile nothing"
    if d == "MayPutExiledLandOntoBattlefield":
        return "Put the land onto the battlefield" if yes else "Leave it in exile"
    d3_no = {"SacrificeAnyNumber": "Done sacrificing", "ReturnAnotherPermanentCard": "Return nothing", "BasicsTappedUpTo": "Find nothing", "SignalTheClans": "Done revealing", "BraidsSacrifice": "Don't sacrifice", "BraidsOpponent": "Don't sacrifice (lose 2 life)"}  # fmt: skip
    d3_no["ReturnSharingType"] = "Return nothing"
    d3_no["ReanimateTotalManaValue"] = "Done choosing"
    d3_no["ExileThreeForFood"] = "Don't exile (no more Food)"
    if not yes and d in d3_no:
        return d3_no[d]
    if d in ("DiscardOrLoseLife", "LoseUnless") and not yes:
        life = f"lose {dec['life']} life" if "life" in dec else "lose the life"
        return f"Don't {'sacrifice' if dec.get('sacrifice') else 'discard'} ({life})"
    if d == "SacrificeDiscardOrLoseLife" and not yes:
        return f"Don't sacrifice or discard (lose {dec.get('life', 3)} life)"
    if d in labels:
        return labels[d][0 if yes else 1]
    if not yes:
        return {"Discard": "Don't discard", "PayWardDiscard": "Don't discard (it's countered)", "LoseOrDiscard": f"Lose {dec.get('life', 0)} life instead",
                "SacrificeAnother": "Don't sacrifice", "Search": "Find nothing", "LookPick": "Take nothing",
                "GenesisPick": "Done choosing", "EnterAsCopy": "Don't copy anything"}.get(d)  # fmt: skip
    return None


def actions(obs: dict, me: int) -> list[dict]:
    dec = obs.get("decision") or {}
    out = []
    for k, ((kind, a, b, c), cards) in enumerate(zip(obs["legal_actions"], obs["legal_cards"], strict=True)):
        values = (a, b, c)
        objects = [values[s] for s in range(3) if cards[s] != -1]
        names = [name(cards[s]) for s in range(3) if cards[s] != -1]
        nums = tuple(values[s] for s in range(3) if cards[s] == -1 and values[s] >= 0)
        if kind in ("Mode", "Amount", "TargetPlayer", "TapForMana"):
            nums = ()
        label = _label(kind, names, nums, me, values)
        if dec.get("kind") in ("ChooseNewTarget", "ChangeTheTarget") and kind in ("Target", "TargetPlayer", "No"):
            what = name(dec["card"][1]) if dec.get("card") else "it"
            if kind == "No":
                label = "Keep the targets"
            elif kind == "TargetPlayer":
                label = f"Change the target of {what} to {'yourself' if a == me else 'your opponent'}"
            else:
                label = f"Change the target of {what} to {names[0] if names else ''}"
        if dec and kind in ("Mode", "Pick", "Yes", "No", "Amount"):
            label = _decision_label(dec, kind, a, names[0] if names else "") or label
            if kind in ("Yes", "No") and not objects:
                oid, subject = _subject(dec)  # the card the yes/no is about (surveil, tuck, ward)
                if oid is not None and dec.get("kind") in (
                    "Surveil",
                    "Scry",
                    "Tuck",
                    "PayWard",
                    "PayWardLife",
                    "PayWardDiscard",
                    "ShockLand",
                    "CommanderToCommandZone",
                    "PayOrCountered",
                    "SplitPiles",
                ):
                    objects, names = [oid], [subject]
        act = {"index": k, "kind": kind, "label": label, "objects": objects, "cards": names}
        if kind == "TargetPlayer":
            act["player"] = a
        elif kind == "TapForMana":
            act["color"] = "WUBRGC"[b] if 0 <= b <= 5 else "C"
        elif kind in ("Mode", "Amount"):
            act["number"] = a
        elif kind == "ActivateX":
            act["number"] = b % 256
        elif nums:
            act["number"] = nums[0]
        out.append(act)
    return out


def _pending(kind: str, who: int, dec: dict | None, choosing: dict | None = None, bottom_left=None) -> dict:
    """pending, plus what a "Decide" is about: choice (May, Surveil, Tuck, ChooseColor, MayPay, ...),
    source (the asking card) with its rules text, the card or cards involved (looked-at cards only
    reach the decider's view), and any options, cost, life or count."""
    out = {"kind": kind, "player": who}
    if kind == "Bottom" and bottom_left is not None:  # London mulligan: cards still to put on the bottom
        out["left"] = bottom_left
    if choosing and kind in ("ChooseTarget", "Divide"):  # what the targets are for
        out["source"] = _ref((choosing["id"], choosing["card"]))
        out["source_text"] = ORACLE.get(out["source"]["name"], "")
        out["spell"] = bool(choosing.get("spell"))
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
            "lands_played": pl["lands_played"], "pool": pl["pool"], "pool_colors": pl.get("pool_colors"),
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
            "transformed": bool(e.get("transformed")),
            "face_down": bool(e.get("face_down")),
        })  # fmt: skip
        if battlefield[-1]["card_id"] == FACE_DOWN:
            battlefield[-1]["card_id"] = None
    combat = obs.get("combat")
    result = None
    if over:
        r = game.result()
        result = {"winner": 0 if r == "p0" else 1 if r == "p1" else None,
                  "outcome": "win" if r in ("p0", "p1") else "draw"}  # fmt: skip
    kind, who = obs["pending"]
    out = {
        "game_id": game_id, "version": version, "seat": seat, "turn": obs["turn"], "step": obs["step"],
        "active": obs["active"], "to_act": None if over else game.to_act(),
        "pending": _pending(kind, who, obs.get("decision"), obs.get("choosing"), obs.get("bottom_left")), "over": over, "result": result,
        "view": {
            "players": players, "battlefield": battlefield,
            "stack": [{"id": i, "name": name(c), "card_id": c, "controller": ctl} for i, c, ctl in obs["stack"]],
            "exile": [_obj(x) for x in obs["exile"]],
            "combat": None if not combat else {"attackers": list(combat["attackers"]),
                                               "blocks": [list(x) for x in combat["blocks"]]},
        },
        "legal_actions": [] if seat != game.to_act() or over else actions(obs, seat),
    }  # fmt: skip
    _commander_view(out, obs)
    _faces(out)
    return out


def _faces(out: dict) -> None:
    """Double-faced cards (docs/api.md): `face` ("front"/"back": back only for a transformed permanent)
    and `back` {name, image} on every such object the view shows."""
    view = out["view"]
    zones = [view["battlefield"], view["stack"], view["exile"]]
    zones += [pl["graveyard"] for pl in view["players"]] + [pl["hand"] or [] for pl in view["players"]]
    zones += [pl.get("command", []) for pl in view["players"]]
    for zone in zones:
        for o in zone:
            transformed = o.pop("transformed", False)
            if o["name"] in BACKS:
                o["face"] = "back" if transformed else "front"
                o["back"] = {"name": BACKS[o["name"]], "image": back_image(o["name"])}


def _commander_view(out: dict, obs: dict) -> None:
    """Commander games only (docs/api.md): each player's command zone and commander damage taken, and
    `commander: true` plus `owner` on every commander object the view shows."""
    owners = dict(obs.get("commanders") or [])
    if not owners:
        return
    view = out["view"]
    for p, (pl, raw) in enumerate(zip(view["players"], obs["players"], strict=True)):
        pl["command"] = [{"id": i, "name": name(c), "card_id": c, "tax": tax, "commander": True, "owner": p}
                         for i, c, tax in raw.get("command", [])]  # fmt: skip
        pl["commander_damage"] = [{"id": i, "name": name(c), "card_id": c, "owner": o, "damage": n}
                                  for i, c, o, n in raw.get("commander_damage", [])]  # fmt: skip
    zones = [view["battlefield"], view["stack"], view["exile"]]
    zones += [pl["graveyard"] for pl in view["players"]] + [pl["hand"] or [] for pl in view["players"]]
    for zone in zones:
        for o in zone:
            if o["id"] in owners:
                o["commander"] = True
                o["owner"] = owners[o["id"]]
