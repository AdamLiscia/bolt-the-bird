"""Recorded decisions in compact flat arrays (Phase 4, P3c), and padded batches back out of them.

One row per decision; objects and moves are concatenated across decisions with offsets
(`obj_off`, `act_off`), so storage is ~1 KB a decision instead of a padded block. `z` is the game's
result from the deciding seat's side (+1 won, -1 lost, 0 draw, timeout or error).
"""

from pathlib import Path

import numpy as np
import torch

from bolt.play_encoding import FEATS


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
        "card": cat("card", np.int16), "zone": cat("zone", np.int8), "feats": cat("feats", np.float16, len(FEATS)),
        "globals": np.asarray([s["enc"]["globals"] for s in steps], dtype=np.float32),
        "act_kind": cat("act_kind", np.int8), "act_ptr": cat("act_ptr", np.int16, 3),
        "act_card": cat("act_card", np.int16, 3), "act_num": cat("act_num", np.float16, 2),
        "act_player": cat("act_player", np.int8),
        "action": np.asarray([s["action"] for s in steps], dtype=np.int16),
        "seat": np.asarray([s["seat"] for s in steps], dtype=np.int8),
        "game": np.asarray([s["game"] for s in steps], dtype=np.int32),
        "z": np.asarray([z(s) for s in steps], dtype=np.int8),
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
    for k in parts[0]:
        if k in ("obj_off", "act_off"):
            offs, base = [np.zeros(1, np.int64)], 0
            for p in parts:
                offs.append(p[k][1:] + base)
                base += p[k][-1]
            out[k] = np.concatenate(offs)
        else:
            out[k] = np.concatenate([p[k] for p in parts])
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
        card[r, :n], zone[r, :n], feats[r, :n] = arr["card"][s:e], arr["zone"][s:e], arr["feats"][s:e]
        obj_pad[r, :n] = False
        s, e = ao[i], ao[i + 1]
        m = e - s
        kind[r, :m], ptr[r, :m], acard[r, :m] = arr["act_kind"][s:e], arr["act_ptr"][s:e], arr["act_card"][s:e]
        num[r, :m], player[r, :m] = arr["act_num"][s:e], arr["act_player"][s:e]
        act_pad[r, :m] = False
    x = {"card": card, "zone": zone, "feats": feats, "obj_pad": obj_pad, "globals": arr["globals"][idx],
         "act_kind": kind, "act_ptr": ptr, "act_card": acard, "act_num": num, "act_player": player,
         "act_pad": act_pad}  # fmt: skip
    x = {k: torch.as_tensor(v, device=device) for k, v in x.items()}
    action = torch.as_tensor(arr["action"][idx].astype(np.int64), device=device)
    z = torch.as_tensor(arr["z"][idx].astype(np.float32), device=device)
    return x, action, z
