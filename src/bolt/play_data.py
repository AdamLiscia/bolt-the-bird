"""Recorded decisions in compact flat arrays (Phase 4, P3c), and padded batches back out of them.

One row per decision; objects and moves are concatenated across decisions with offsets
(`obj_off`, `act_off`), so storage is ~1 KB a decision instead of a padded block. `z` is the game's
result from the deciding seat's side (+1 won, -1 lost, 0 draw, timeout or error).
"""

from pathlib import Path

import numpy as np
import torch

from bolt.play_encoding import FEATS, GLOBAL_NAMES

F16_MAX = 60000.0


def f16(a: np.ndarray) -> np.ndarray:
    """float16 storage, clipped so a huge number (a power doubled turn after turn) stays finite, not inf."""
    return np.clip(a, -F16_MAX, F16_MAX).astype(np.float16)


def to_arrays(steps: list[dict], results: list[dict]) -> dict:
    n_obj = [len(s["enc"]["card"]) for s in steps]
    n_act = [len(s["enc"]["act_kind"]) for s in steps]

    def cat(key, dtype, width=None):
        rows = [r for s in steps for r in s["enc"][key]]
        shape = (0, width) if width else (0,)
        return np.asarray(rows, dtype=dtype) if rows else np.zeros(shape, dtype=dtype)

    def z(s):
        r = results[s["game"]]["result"]
        return 0 if r not in ("p0", "p1") else (1 if r == f"p{s['seat']}" else -1)

    return {
        "obj_off": np.concatenate([[0], np.cumsum(n_obj)]).astype(np.int64),
        "act_off": np.concatenate([[0], np.cumsum(n_act)]).astype(np.int64),
        "card": cat("card", np.int16), "zone": cat("zone", np.int8), "feats": f16(cat("feats", np.float32, len(FEATS))),
        # (0, n) when no step was recorded (every game of a matchup dropped), so it concatenates with the rest
        "globals": np.asarray([s["enc"]["globals"] for s in steps], dtype=np.float32).reshape(-1, len(GLOBAL_NAMES)),
        "act_kind": cat("act_kind", np.int8), "act_ptr": cat("act_ptr", np.int16, 3),
        "act_card": cat("act_card", np.int16, 3), "act_num": f16(cat("act_num", np.float32, 2)),
        "act_player": cat("act_player", np.int8),
        "action": np.asarray([s["action"] for s in steps], dtype=np.int16),
        "seat": np.asarray([s["seat"] for s in steps], dtype=np.int8),
        "game": np.asarray([s["game"] for s in steps], dtype=np.int32),
        "z": np.asarray([z(s) for s in steps], dtype=np.int8),
        # the network's own probability of the move it played and its value estimate (NaN for teacher steps)
        "logp": np.asarray([np.nan if s.get("logp") is None else s["logp"] for s in steps], dtype=np.float32),
        "value": np.asarray([np.nan if s.get("value") is None else s["value"] for s in steps], dtype=np.float32),
    }  # fmt: skip


def save(path, arr: dict) -> None:
    path = Path(path)
    tmp = path.with_suffix(".tmp.npz")
    np.savez(tmp, **arr)
    tmp.replace(path)  # a crash mid-write never leaves a half shard under the real name


def load(path) -> dict:
    with np.load(path) as f:
        return {k: f[k] for k in f.files}


def concat(parts: list[dict]) -> dict:
    out = {}
    keys = [k for k in parts[0] if all(k in p for p in parts)]  # older shards lack logp/value
    for k in keys:
        if k in ("obj_off", "act_off"):
            offs, base = [np.zeros(1, np.int64)], 0
            for p in parts:
                offs.append(p[k][1:] + base)
                base += p[k][-1]
            out[k] = np.concatenate(offs)
        else:
            out[k] = np.concatenate([p[k] for p in parts])
    return out


BIG_OBJECTS, BIG_MOVES = 128, 256
GROUP_ROWS = 64
GROUP_BUDGET = GROUP_ROWS * (BIG_OBJECTS + BIG_MOVES)  # rows x (objects + moves) of padding a group may hold
# and rows x objects^2: attention is quadratic in objects, so a 559-object board shares with at most 2 others
GROUP_ATTENTION = GROUP_ROWS * BIG_OBJECTS**2


def finite_rows(arr: dict) -> np.ndarray:
    """Per decision: True when everything the update reads for it is finite (its objects' features, its moves'
    numbers, its globals, and its logp, value, adv and ret where present)."""
    n = len(arr["action"])
    ok = np.ones(n, bool)
    for k in ("logp", "value", "adv", "ret"):
        if k in arr:
            ok &= np.isfinite(arr[k].astype(np.float64))
    ok &= np.isfinite(arr["globals"]).all(1)
    for key, off in (("feats", "obj_off"), ("act_num", "act_off")):
        bad = (~np.isfinite(arr[key].astype(np.float32))).any(1).astype(np.int64)
        cs = np.concatenate([[0], np.cumsum(bad)])
        ok &= (cs[arr[off][1:]] - cs[arr[off][:-1]]) == 0
    return ok


def within(arr: dict, max_objects: int) -> np.ndarray:
    """The decisions (indices) whose positions have at most max_objects objects (0 = all of them)."""
    n_obj = np.diff(arr["obj_off"])
    return np.flatnonzero(n_obj <= max_objects) if max_objects else np.arange(len(n_obj))


def groups(arr, idx):
    """idx split into groups of similar size, so padding stays small: rows sorted by object then move count,
    at most GROUP_ROWS a group, and fewer when they're big (token-filled boards: up to ~460 objects, ~1,600
    moves), keeping rows x the largest (objects + moves) within GROUP_BUDGET and rows x the largest objects^2
    within GROUP_ATTENTION. Every row lands in one group,
    and the loss is the same however rows are grouped (ppo_step weights each group by its size).

    2026-11-27: Commander positions (median 41 objects and 2 moves, a 99th percentile of 133 and 83) padded
    to the batch's largest made the update 90% of a cmd_play_1 iteration; sorted groups of 64 cut a
    1,024-decision step from 7.0 s to 2.1 s with identical losses."""
    idx = np.asarray(idx)
    n_obj = arr["obj_off"][idx + 1] - arr["obj_off"][idx]
    n_act = arr["act_off"][idx + 1] - arr["act_off"][idx]
    order = np.lexsort((n_act, n_obj))
    out, cur, widest, deepest = [], [], 0, 0
    for k in order:
        w, d = max(widest, int(n_obj[k] + n_act[k])), max(deepest, int(n_obj[k]))
        if cur and (
            len(cur) >= GROUP_ROWS or (len(cur) + 1) * w > GROUP_BUDGET or (len(cur) + 1) * d * d > GROUP_ATTENTION
        ):
            out.append(idx[cur])
            cur, w, d = [], int(n_obj[k] + n_act[k]), int(n_obj[k])
        cur.append(k)
        widest, deepest = w, d
    if cur:
        out.append(idx[cur])
    return out


def batch(arr: dict, idx, device="cpu"):
    """(x for PlayNet, teacher action (B,), result z (B,)) for the decisions at idx."""
    idx = np.asarray(idx)
    oo, ao = arr["obj_off"], arr["act_off"]
    n_obj = oo[idx + 1] - oo[idx]
    n_act = ao[idx + 1] - ao[idx]
    b, t, a = len(idx), max(1, int(n_obj.max())), int(n_act.max())
    card = np.zeros((b, t), np.int64)
    zone = np.zeros((b, t), np.int64)
    feats = np.zeros((b, t, len(FEATS)), np.float32)
    obj_pad = np.ones((b, t), bool)
    kind = np.zeros((b, a), np.int64)
    ptr = np.full((b, a, 3), -1, np.int64)
    acard = np.zeros((b, a, 3), np.int64)
    num = np.zeros((b, a, 2), np.float32)
    player = np.zeros((b, a), np.int64)
    act_pad = np.ones((b, a), bool)
    for r, i in enumerate(idx):
        s, e = oo[i], oo[i + 1]
        n = e - s
        # shards recorded before features were appended are narrower: the new ones stay zero
        card[r, :n], zone[r, :n] = arr["card"][s:e], arr["zone"][s:e]
        feats[r, :n, : arr["feats"].shape[1]] = arr["feats"][s:e]
        obj_pad[r, :n] = False
        s, e = ao[i], ao[i + 1]
        m = e - s
        kind[r, :m], ptr[r, :m], acard[r, :m] = arr["act_kind"][s:e], arr["act_ptr"][s:e], arr["act_card"][s:e]
        num[r, :m], player[r, :m] = arr["act_num"][s:e], arr["act_player"][s:e]
        act_pad[r, :m] = False
    glob = np.zeros((b, len(GLOBAL_NAMES)), np.float32)
    glob[:, : arr["globals"].shape[1]] = arr["globals"][idx]
    x = {"card": card, "zone": zone, "feats": feats, "obj_pad": obj_pad, "globals": glob,
         "act_kind": kind, "act_ptr": ptr, "act_card": acard, "act_num": num, "act_player": player,
         "act_pad": act_pad}  # fmt: skip
    x = {k: torch.as_tensor(v, device=device) for k, v in x.items()}
    action = torch.as_tensor(arr["action"][idx].astype(np.int64), device=device)
    z = torch.as_tensor(arr["z"][idx].astype(np.float32), device=device)
    return x, action, z
