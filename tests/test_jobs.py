"""bolt.jobs: power-cut-safe checkpoints, resume and idempotent batches, against an in-memory S3."""

import io
import json
import signal
from pathlib import Path

import pytest

torch = pytest.importorskip("torch")

from bolt.jobs import Job  # noqa: E402


class FakeS3:
    """put/head/get like boto3's S3 client; `fail_puts` makes the next N puts raise (a dropped link)."""

    def __init__(self):
        self.objects, self.fail_puts = {}, 0

    def put_object(self, Bucket, Key, Body):
        if self.fail_puts:
            self.fail_puts -= 1
            raise ConnectionError("dropped")
        self.objects[Key] = bytes(Body)

    def head_object(self, Bucket, Key):
        return {"ContentLength": len(self.objects[Key])}

    def get_object(self, Bucket, Key):
        return {"Body": io.BytesIO(self.objects[Key])}


def job(tmp_path, s3, name="run1"):
    return Job(name, every_s=900, client=s3, local=tmp_path / "ckpt")


def test_save_then_resume_returns_the_newest_state(tmp_path):
    s3 = FakeS3()
    j = job(tmp_path, s3)
    assert j.resume() is None, "nothing saved: start fresh"
    j.save({"w": torch.tensor([1.0])}, step=1)
    j.save({"w": torch.tensor([2.0])}, step=2)
    got = job(tmp_path, s3).resume()
    assert got["step"] == 2 and float(got["w"]) == 2.0
    pointer = json.loads(s3.objects["farm/run1/latest.json"])
    assert pointer["step"] == 2 and len(pointer["history"]) == 1


def test_a_lost_disk_resumes_from_s3(tmp_path):
    s3 = FakeS3()
    job(tmp_path, s3).save({"w": torch.tensor([3.0])}, step=7)
    other = Job("run1", client=s3, local=tmp_path / "fresh_disk")
    assert other.resume()["step"] == 7


def test_a_corrupt_newest_checkpoint_falls_back_to_the_one_before(tmp_path):
    s3 = FakeS3()
    j = job(tmp_path, s3)
    j.save({"w": torch.tensor([1.0])}, step=1)
    key = j.save({"w": torch.tensor([2.0])}, step=2)
    s3.objects[key] = b"garbage"
    (tmp_path / "ckpt" / key.rsplit("/", 1)[1]).write_bytes(b"garbage")
    assert job(tmp_path, s3).resume()["step"] == 1


def test_a_dropped_upload_is_retried_and_the_pointer_waits_for_it(tmp_path, monkeypatch):
    monkeypatch.setattr("bolt.jobs.time.sleep", lambda s: None)
    s3 = FakeS3()
    j = job(tmp_path, s3)
    s3.fail_puts = 2  # the checkpoint upload fails twice, then lands
    j.save({"w": torch.tensor([5.0])}, step=3)
    assert json.loads(s3.objects["farm/run1/latest.json"])["step"] == 3


def test_an_upload_that_never_lands_leaves_the_old_pointer(tmp_path, monkeypatch):
    monkeypatch.setattr("bolt.jobs.time.sleep", lambda s: None)
    s3 = FakeS3()
    j = job(tmp_path, s3)
    j.save({"w": torch.tensor([1.0])}, step=1)
    s3.fail_puts = 99
    with pytest.raises(ConnectionError):
        j.save({"w": torch.tensor([2.0])}, step=2)
    assert json.loads(s3.objects["farm/run1/latest.json"])["step"] == 1


def test_batches_are_recorded_after_upload_and_rerunning_one_overwrites(tmp_path):
    s3 = FakeS3()
    j = job(tmp_path, s3)
    state = {}
    j.batch_done(state, "b0", {"wins": 3})
    j.batch_done(state, "b0", {"wins": 3})  # the same batch re-run after a cut
    j.batch_done(state, "b1", {"wins": 5})
    assert state["done_batches"] == ["b0", "b1"]
    total = sum(r["wins"] for r in j.results(state["done_batches"]).values())
    assert total == 8, "recomputed from the result files, never double-counted"


def test_done_marks_the_job_finished(tmp_path):
    s3 = FakeS3()
    j = job(tmp_path, s3)
    j.save({}, step=1)
    assert not j.done()
    j.save({}, step=2, done=True)
    assert job(tmp_path, s3).done()


def test_a_pointer_check_leaves_the_callers_sigterm_handler(tmp_path):
    """farm.py checks each job's pointer with a Job; that must not take over farm.py's SIGTERM handler."""

    def farm_stop(*_):
        pass

    old = signal.signal(signal.SIGTERM, farm_stop)
    try:
        Job("run1", client=FakeS3(), local=tmp_path / "ckpt", handle_signals=False).done()
        assert signal.getsignal(signal.SIGTERM) is farm_stop
        Job("run1", client=FakeS3(), local=tmp_path / "ckpt")
        assert signal.getsignal(signal.SIGTERM) is not farm_stop, "a training job still saves on SIGTERM"
    finally:
        signal.signal(signal.SIGTERM, old)


def test_farm_checks_pointers_without_taking_signals():
    farm = (Path(__file__).resolve().parents[1] / "scripts" / "farm.py").read_text()
    assert 'Job(j["name"], handle_signals=False)' in farm
