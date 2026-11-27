"""Every farm job file's command line parses with the script it runs (2026-11-27: a flag put in
jobs/cmd_play_1.json before the trainer defined it stopped the desktop farm for an hour)."""

import importlib.util
import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
JOBS = sorted((ROOT / "jobs").glob("*.json"))


def _parser(script: Path):
    spec = importlib.util.spec_from_file_location(script.stem, script)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.parser()


@pytest.mark.parametrize("job", JOBS, ids=[j.stem for j in JOBS])
def test_job_command_line_parses(job):
    cmd = json.loads(job.read_text())["cmd"]
    script = next(Path(c) for c in cmd if c.endswith(".py"))
    if script.name != "train_ppo_play.py":
        pytest.skip(f"no parser() check for {script.name} yet")
    pytest.importorskip("torch")
    args = cmd[cmd.index(str(script)) + 1 :]
    _parser(ROOT / script).parse_args(args)  # an unknown flag exits with code 2


def test_a_flag_the_trainer_lacks_is_caught():
    pytest.importorskip("torch")
    with pytest.raises(SystemExit):
        _parser(ROOT / "scripts" / "train_ppo_play.py").parse_args(["--no-such-flag", "1"])
