"""train_ppo_play: a printed iteration is a saved one. A SIGTERM right after an iteration's line (2026-11-27: the
farm resumed at iteration 7 although 8 had printed) resumes on that iteration, not the one before."""

import sys

import numpy as np
import pytest
import torch
from test_jobs import FakeS3
from test_play_commander import obs

from bolt.jobs import Job
from bolt.play_data import to_arrays
from bolt.play_encoding import encode
from bolt.play_net import PlayNet


def fake_run_parallel(net, jobs, record=(0, 1), stop=None, **kw):
    """Every game a two-move draw-free win for seat 0, both seats recorded, played by no engine."""
    steps, results = [], []
    for g in range(len(jobs)):
        for seat in record:
            for _ in range(2):
                steps.append({"game": g, "seat": seat, "enc": encode(obs(), seat), "action": 1, "logp": -0.7,
                              "value": 0.0})  # fmt: skip
        results.append({"result": "p0", "turn": 3})
    arr = to_arrays(steps, results)
    if len(arr["logp"]):
        arr["logp"][0] = np.nan  # one decision with a non-finite input: dropped, not fatal
    return {"results": results, "arrays": arr}


@pytest.fixture
def trainer(tmp_path, monkeypatch):
    sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
    import train_ppo_play as t

    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: ())
    torch.manual_seed(0)
    (tmp_path / "results").mkdir()
    init = tmp_path / "init.pt"
    net = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    torch.save({"config": net.config, "model": net.state_dict()}, init)
    monkeypatch.setattr(t, "ROOT", tmp_path)
    monkeypatch.setattr(t, "PLAY_SET", None)
    monkeypatch.setattr(t, "deck_sets", lambda: ([[1], [2], [3]], [[1], [2], [3]]))
    monkeypatch.setattr(t, "run_parallel", fake_run_parallel)
    monkeypatch.setattr(t, "REAL_EVALUATE", t.evaluate, raising=False)
    monkeypatch.setattr(t, "evaluate", lambda *a, **k: {"rules": 0.5, "smart": 0.5})
    monkeypatch.setattr(t, "upload_quietly", lambda p: None)
    s3, made = FakeS3(), []

    def make_job(name):
        j = Job(name, every_s=900, client=s3, local=tmp_path / "ckpt", handle_signals=False)
        made.append(j)
        return j

    monkeypatch.setattr("bolt.jobs.Job", make_job)
    monkeypatch.setenv("BOLT_JOB", "stoptest")
    monkeypatch.delenv("BOLT_THREADS", raising=False)
    argv = ["train_ppo_play.py", "--name", "stoptest", "--init", str(init), "--self-play", "--iters", "5",
            "--games", "4", "--workers", "1", "--threads", "1", "--eval-every", "100", "--batch", "8"]  # fmt: skip
    monkeypatch.setattr(sys, "argv", argv)
    return t, made, s3, tmp_path


def test_a_sigterm_right_after_an_iterations_line_resumes_on_that_iteration(trainer, monkeypatch, capsys):
    t, made, s3, tmp_path = trainer
    real_print = print
    armed = []

    def printing(*args, **kw):
        real_print(*args, **kw)
        if args and str(args[0]).startswith('{"iter": 2,'):
            armed.append(True)

    def playing(*args, stop=None, **kw):
        if armed:  # the SIGTERM lands while iteration 3's games play, seconds after iteration 2's line
            made[0].stop_requested = True  # what Job's SIGTERM handler does
            if stop is not None and stop():
                raise t.Stopped()
        return fake_run_parallel(*args, **kw)

    monkeypatch.setattr(t, "print", printing, raising=False)
    monkeypatch.setattr(t, "run_parallel", playing)
    t.main()
    out = capsys.readouterr().out
    assert '"iter": 3' not in out, "the iteration the SIGTERM interrupted is dropped, not finished"
    assert "stopped at iteration 2 (SIGTERM); checkpoint saved" in out
    back = Job("stoptest", client=s3, local=tmp_path / "elsewhere", handle_signals=False).resume()
    assert back is not None and back["state"]["iter"] == 2
    assert np.isclose(back["state"]["history"][-1]["iter"], 2)


def test_every_iteration_row_reports_entropy_and_ratio_deviation(trainer, monkeypatch):
    import json

    t, made, s3, tmp_path = trainer
    monkeypatch.setattr(sys, "argv", [*sys.argv[:7], "2", *sys.argv[8:]])  # --iters 2
    t.main()
    rows = json.loads((tmp_path / "results" / "ppo_play_stoptest.json").read_text())["history"]
    assert len(rows) == 2
    for r in rows:
        assert r["entropy"] > 0 and r["ratio_dev"] >= 0 and r["value_mse_z"] >= 0, r
        assert 0 < r["entropy_norm"] <= 1.0001 and r["log_legal"] > 0, r
        assert r["by_kind"]["Priority"]["share"] == 1.0, r["by_kind"]
        assert r["dropped_nan"] == 1 and r["skipped_nan"] == 0 and r["dropped_error"] == 0, r


def test_a_changed_lr_applies_to_the_resumed_optimizer(monkeypatch):
    sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
    import train_ppo_play as t

    monkeypatch.setattr(t, "PLAY_SET", None)
    net = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    opt = torch.optim.AdamW(net.parameters(), lr=2e-5)
    net.value.weight.grad = torch.ones_like(net.value.weight)
    opt.step()
    saved = t.snapshot(net, opt, {"iter": 20})
    net2 = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    opt2 = torch.optim.AdamW(net2.parameters(), lr=5e-5)
    t.resume_into(net2, opt2, saved, lr=5e-5)
    assert [g["lr"] for g in opt2.param_groups] == [5e-5], "load_state_dict alone would restore 2e-5"
    m, m2 = opt.state[net.value.weight]["exp_avg"], opt2.state[net2.value.weight]["exp_avg"]
    assert torch.equal(m, m2), "Adam's moments carry over"


def test_an_errored_past_checkpoint_game_is_counted_in_the_row(trainer, monkeypatch):
    """2026-11-27, iteration 30: a panic in an opp/net game (the net in seat 1) and a row with dropped_error 0.
    Here the real run_games plays every game in-process; seat layouts come from the real league."""
    import json

    from test_play_hang import CrawlingGame, PanickingGame

    import bolt.play_runner as pr

    t, made, s3, tmp_path = trainer
    real_encode = __import__("bolt.play_encoding", fromlist=["encode"]).encode
    monkeypatch.setattr(pr, "encode", lambda o, who: real_encode(obs(), who))
    layouts = []

    def in_process(net, jobs, stop=None, workers=1, chunk=None, **kw):
        layouts.append(tuple(kw["seats"]))
        return pr._worker((pr._state(net), jobs, {**kw, "opp_net": pr._state(kw.get("opp_net")), "game_s": 0}))

    def new_game(a, b, seed, first):  # iteration 2, the ("opp", "net") matchup's first game panics
        return PanickingGame() if seed == 20_000_000 + 2 * 10_000 + 1_000 else CrawlingGame(False)

    monkeypatch.setattr(pr, "new_game", new_game)
    monkeypatch.setattr(t, "run_parallel", in_process)
    monkeypatch.setattr(sys, "argv", [*sys.argv[:7], "2", *sys.argv[8:], "--past-every", "1"])  # 2 iterations
    t.main()
    assert ("opp", "net") in layouts
    rows = json.loads((tmp_path / "results" / "ppo_play_stoptest.json").read_text())["history"]
    r = rows[1]
    assert r["errors"] == 1
    assert r["dropped_error"] == 3, "the net seat's 3 decisions before the panic"
    games = [json.loads(x) for x in (tmp_path / "results" / "games_stoptest.jsonl").read_text().splitlines()]
    assert all("end_reason" in g and "cards_played" in g and "loop_capped" in g for g in games)
    assert [g["end_reason"] for g in games if g["error"]] == ["error"]


def test_ref_sets_the_kl_reference_and_a_resume_keeps_it(trainer, monkeypatch, capsys):
    t, made, s3, tmp_path = trainer
    torch.manual_seed(7)
    other = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    ref_path = tmp_path / "ref.pt"
    torch.save({"config": other.config, "model": other.state_dict()}, ref_path)
    loaded, refs_used = [], []
    real_load, real_step = t.load_net, t.ppo_step

    def load_net(path):
        loaded.append(str(path))
        return real_load(path)  # load_checkpoint on today's pool (BOLT_PLAY_SET), like --init

    def ppo_step(net, ref_net, *args, **kw):
        refs_used.append(ref_net)
        return real_step(net, ref_net, *args, **kw)

    monkeypatch.setattr(t, "load_net", load_net)
    monkeypatch.setattr(t, "ppo_step", ppo_step)
    argv = [*sys.argv[:7], "1", *sys.argv[8:], "--ref", str(ref_path)]
    monkeypatch.setattr(sys, "argv", argv)
    t.main()
    assert str(ref_path) in loaded and f"KL reference: {ref_path}" in capsys.readouterr().out
    assert refs_used and all(torch.equal(r.value.weight, other.value.weight) for r in refs_used)
    # the same run resumed without --ref keeps it
    loaded.clear()
    monkeypatch.setattr(sys, "argv", [*sys.argv[:7], "1", *sys.argv[8:-2]])
    t.main()
    assert str(ref_path) in loaded and f"KL reference: {ref_path}" in capsys.readouterr().out


def test_without_ref_the_kl_reference_is_init(trainer, monkeypatch, capsys):
    t, made, s3, tmp_path = trainer
    monkeypatch.setattr(sys, "argv", [*sys.argv[:7], "1", *sys.argv[8:]])
    t.main()
    assert f"KL reference: {tmp_path / 'init.pt'}" in capsys.readouterr().out


def test_every_game_is_logged_with_its_decks_and_phase(trainer, monkeypatch):
    import json

    t, made, s3, tmp_path = trainer
    monkeypatch.setattr(t, "evaluate", t.REAL_EVALUATE)  # the real one, over the fake games
    argv = [*sys.argv[:7], "1", *sys.argv[8:]]
    i = argv.index("--eval-every")
    argv[i + 1] = "1"
    monkeypatch.setattr(sys, "argv", [*argv, "--eval-pairs", "2"])
    t.main()
    lines = [json.loads(x) for x in (tmp_path / "results" / "games_stoptest.jsonl").read_text().splitlines()]
    phases = [(x["iter"], x["phase"]) for x in lines]
    # eval 0: 2 pairs x 2 seat orders, against rules then smart; iteration 1: 4 self-play games; then eval 1
    assert (
        phases
        == [(0, "eval_rules")] * 4
        + [(0, "eval_smart")] * 4
        + [(1, "self")] * 4
        + [(1, "eval_rules")] * 4
        + [(1, "eval_smart")] * 4
    )
    ev = [x for x in lines if x["phase"] == "eval_rules" and x["iter"] == 0]
    assert [(x["deck_a"]["key"], x["deck_b"]["key"], x["seed"], x["first"]) for x in ev] == [
        (0, 1, 9000, 0),
        (1, 2, 9001, 1),
    ] * 2
    assert [x["seats"] for x in ev] == [["net", "rules"]] * 2 + [["rules", "net"]] * 2
    for x in lines:
        assert x["deck_a"]["key"] != x["deck_b"]["key"] and x["result"] == "p0" and x["turn"] == 3
        assert x["timeout"] is False and x["error"] is False and "moves" in x
