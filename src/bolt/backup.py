"""Back up checkpoints to Adam's S3 bucket (put-only: the credentials can't delete, by design).

On by default (Adam approved the bucket 2026-11-25); BOLT_BACKUP=0 turns it off for a run.

    python -m bolt.backup sync [--until 14:30] [--dry-run]

Training scripts call `upload_quietly(path)` after saving a checkpoint; it never raises, so a failed
upload can't fail a run. Every attempt is logged to checkpoints/UPLOADED.jsonl.
"""

import contextlib
import datetime as dt
import json
import os
import sys
import threading
from pathlib import Path

from bolt.data import ROOT

CK = ROOT / "checkpoints"
BUCKET = "nerdleagues-bolt-checkpoints-011834"
PROFILE = "bolt-backup"
MANIFEST = CK / "UPLOADED.jsonl"
_lock = threading.Lock()


def enabled() -> bool:
    return os.environ.get("BOLT_BACKUP", "1") != "0"


def _client():
    import boto3  # imported lazily: training must not need boto3 when backup is off
    from botocore.config import Config

    return boto3.Session(profile_name=PROFILE).client("s3", config=Config(retries={"max_attempts": 5}))


def _log(rec: dict) -> None:
    rec["ts"] = dt.datetime.now().isoformat(timespec="seconds")
    with _lock, MANIFEST.open("a") as f:
        f.write(json.dumps(rec) + "\n")


def _key(path: Path) -> str:
    return "checkpoints/" + path.resolve().relative_to(CK.resolve()).as_posix()


def upload(path: Path, client=None) -> None:
    from boto3.s3.transfer import TransferConfig

    client = client or _client()
    cfg = TransferConfig(multipart_threshold=64 << 20, multipart_chunksize=64 << 20, max_concurrency=2)
    client.upload_file(str(path), BUCKET, _key(path), Config=cfg)
    _log({"key": _key(path), "bytes": path.stat().st_size, "ok": True})


def upload_quietly(path) -> None:
    """Upload one checkpoint if backup is on. Never raises."""
    if not enabled():
        return
    path = Path(path)
    try:
        upload(path)
    except Exception as e:  # noqa: BLE001 - a backup failure must never stop training
        with contextlib.suppress(Exception):
            _log({"key": str(path), "ok": False, "error": f"{type(e).__name__}: {e}"[:300]})


def _remote_sizes(client) -> dict[str, int]:
    sizes, token = {}, None
    while True:
        kw = {"Bucket": BUCKET, "Prefix": "checkpoints/"} | ({"ContinuationToken": token} if token else {})
        r = client.list_objects_v2(**kw)
        sizes |= {o["Key"]: o["Size"] for o in r.get("Contents", [])}
        if not r.get("IsTruncated"):
            return sizes
        token = r["NextContinuationToken"]


def sync(until: str | None, dry_run: bool) -> None:
    files = sorted(p for p in CK.rglob("*") if p.is_file() and p.name != MANIFEST.name)
    if dry_run:
        print(f"{len(files)} files, {sum(p.stat().st_size for p in files) / 2**30:.2f} GiB (dry run, no network)")
        return
    if not enabled():
        sys.exit("backup is off for this run (BOLT_BACKUP=0)")
    client = _client()
    remote = _remote_sizes(client)
    stop = None
    if until:
        h, m = map(int, until.split(":"))
        stop = dt.datetime.now().replace(hour=h, minute=m, second=0, microsecond=0)
    done = skipped = failed = 0
    sent = 0
    for p in files:
        if stop and dt.datetime.now() >= stop:
            print(f"paused at {until}: rerun after the stream to continue")
            break
        if remote.get(_key(p)) == p.stat().st_size:
            skipped += 1
            continue
        try:
            upload(p, client)
            done += 1
            sent += p.stat().st_size
        except Exception as e:  # noqa: BLE001
            failed += 1
            _log({"key": _key(p), "ok": False, "error": f"{type(e).__name__}: {e}"[:300]})
    print(f"uploaded {done} ({sent / 2**30:.2f} GiB), already there {skipped}, failed {failed}")


if __name__ == "__main__":
    args = sys.argv[1:]
    if not args or args[0] != "sync":
        sys.exit(__doc__)
    until = args[args.index("--until") + 1] if "--until" in args else None
    sync(until, "--dry-run" in args)
