"""Checkpoints that survive a power cut, for the desktop farm (docs/desktop-farm.md).

    job = Job("tdm_play_1")
    state = job.resume()             # the newest good checkpoint, or None to start fresh
    while ...:
        ... one step ...
        if job.due():                # every `every_s` seconds (wall clock), or after SIGTERM
            job.save(state, step)
    job.save(state, step, done=True)

A checkpoint is written locally (temp file, fsync, rename), uploaded under a new unique key, confirmed with
a HEAD request, and only then pointed to by `latest.json` (locally and on S3). The pointer never names a
half-written checkpoint, and nothing is ever deleted (the farm's S3 key can't delete).

Simulation batches: `job.batch_done(state, batch_id, results)` uploads a batch's results under their own key
(`results/<batch_id>.json`), confirms it, and only then records the ID in `state["done_batches"]`. A batch
cut halfway re-runs whole with the same seeds and overwrites the same key, so nothing is counted twice.
"""

import contextlib
import hashlib
import io
import json
import os
import random
import signal
import time
from pathlib import Path

import numpy as np

from bolt.data import ROOT

BUCKET = "nerdleagues-bolt-checkpoints-011834"
HISTORY = 5  # earlier checkpoint keys kept in the pointer, to fall back to if the newest is bad


def _client(profile: str):
    import boto3
    from botocore.config import Config

    return boto3.Session(profile_name=profile).client("s3", config=Config(retries={"max_attempts": 8}))


def rng_state() -> dict:
    """Python's, NumPy's and torch's generator states, so a resumed run continues the same stream."""
    import torch

    return {"python": random.getstate(), "numpy": np.random.get_state(), "torch": torch.get_rng_state()}


def set_rng_state(s: dict) -> None:
    import torch

    random.setstate(s["python"])
    np.random.set_state(s["numpy"])
    torch.set_rng_state(s["torch"])


def _sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


class Job:
    def __init__(self, run: str, every_s: float = 900, profile: str | None = None, client=None,
                 local: Path | None = None, bucket: str = BUCKET):  # fmt: skip
        self.run, self.every_s, self.bucket = run, every_s, bucket
        self.prefix = f"farm/{run}"
        self.local = local or ROOT / "ckpt" / run
        self.s3 = client if client is not None else _client(profile or os.environ.get("AWS_PROFILE", "bolt-desktop"))
        self.last = time.time()
        self.stop_requested = False
        with contextlib.suppress(ValueError):  # not the main thread: the caller handles signals
            signal.signal(signal.SIGTERM, self._on_term)

    def _on_term(self, *_):
        self.stop_requested = True  # save at the next step boundary, then the caller exits

    def due(self) -> bool:
        return self.stop_requested or time.time() - self.last >= self.every_s

    # ---- S3, with retries on a dropped connection (a long SIGSTOP pause) ----

    def _put(self, key: str, data: bytes, tries: int = 5) -> None:
        for k in range(tries):
            try:
                self.s3.put_object(Bucket=self.bucket, Key=key, Body=data)
                head = self.s3.head_object(Bucket=self.bucket, Key=key)
                if head["ContentLength"] == len(data):
                    return
            except Exception:  # noqa: BLE001 - retried; the pointer is only written after success
                if k == tries - 1:
                    raise
            time.sleep(min(60, 2**k))
        raise OSError(f"upload of {key} never confirmed")

    def _get(self, key: str) -> bytes | None:
        try:
            return self.s3.get_object(Bucket=self.bucket, Key=key)["Body"].read()
        except Exception:  # noqa: BLE001 - a missing key or an unreachable S3 means "not there"
            return None

    # ---- checkpoints ----

    def save(self, state: dict, step: int, done: bool = False) -> str:
        import torch

        buf = io.BytesIO()
        torch.save({**state, "step": step, "job": self.run, "time": time.time()}, buf)
        data = buf.getvalue()
        name = f"step_{step}_{time.strftime('%Y%m%dT%H%M%S', time.gmtime())}.pt"
        _write_atomic(self.local / name, data)
        key = f"{self.prefix}/{name}"
        self._put(key, data)  # confirmed (HEAD) before anything points to it
        old = self._pointer() or {}
        history = ([old["key"]] if old.get("key") else []) + old.get("history", [])
        pointer = {"key": key, "step": step, "sha256": _sha(data), "time": time.time(), "done": done,
                   "history": history[:HISTORY]}  # fmt: skip
        raw = json.dumps(pointer).encode()
        self._put(f"{self.prefix}/latest.json", raw)
        _write_atomic(self.local / "latest.json", raw)
        self.last = time.time()
        return key

    def _pointer(self) -> dict | None:
        """The newer of the local and S3 pointers (a lost disk still resumes from S3)."""
        found = []
        p = self.local / "latest.json"
        if p.exists():
            with contextlib.suppress(ValueError):
                found.append(json.loads(p.read_text()))
        raw = self._get(f"{self.prefix}/latest.json")
        if raw:
            with contextlib.suppress(ValueError):
                found.append(json.loads(raw))
        return max(found, key=lambda x: x.get("step", -1)) if found else None

    def done(self) -> bool:
        p = self._pointer()
        return bool(p and p.get("done"))

    def resume(self) -> dict | None:
        """The newest checkpoint whose bytes match its pointer, or None to start fresh."""
        import torch

        p = self._pointer()
        if not p:
            return None
        for i, key in enumerate([p["key"], *p.get("history", [])]):
            local = self.local / key.rsplit("/", 1)[1]
            data = local.read_bytes() if local.exists() else self._get(key)
            if data is None or (i == 0 and _sha(data) != p["sha256"]):
                continue  # missing or corrupt: fall back to the one before
            try:
                return torch.load(io.BytesIO(data), map_location="cpu", weights_only=False)
            except Exception:  # noqa: BLE001 - an unreadable file is skipped like a missing one
                continue
        return None

    # ---- idempotent simulation batches ----

    def batch_done(self, state: dict, batch_id: str, results) -> None:
        """Upload a batch's results under their own key, confirm, then record it as done."""
        self._put(f"{self.prefix}/results/{batch_id}.json", json.dumps(results).encode())
        state.setdefault("done_batches", [])
        if batch_id not in state["done_batches"]:
            state["done_batches"].append(batch_id)

    def results(self, batch_ids) -> dict:
        """Every recorded batch's results, read back from S3: totals are recomputed from these."""
        out = {}
        for b in batch_ids:
            raw = self._get(f"{self.prefix}/results/{b}.json")
            if raw is not None:
                out[b] = json.loads(raw)
        return out
