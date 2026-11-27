"""run_parallel's watchdog: a worker stuck in one engine call is killed, and its chunk rerun without that game."""

import json
import time

import pytest
from fake_play_worker import worker

from bolt.play_runner import run_parallel


def test_a_hung_game_is_a_timeout_and_the_rest_of_its_chunk_still_plays(tmp_path):
    jobs = [(f"deck{s}", "other", s, s % 2) for s in (1, 2, 666, 4, 777)]
    log = tmp_path / "hangs.jsonl"
    t = time.time()
    out = run_parallel(None, jobs, workers=2, chunk=3, hang_s=2, hang_log=str(log), worker=worker,
                       seats=("net", "net"))  # fmt: skip
    assert time.time() - t < 60
    assert [r["result"] for r in out["results"]] == ["p0", "p0", "timeout", "p0", "timeout"]
    assert out["results"][2].get("hung")
    rows = [json.loads(x) for x in log.read_text().splitlines()]
    assert [r["seed"] for r in rows] == [666, 777]
    assert rows[0]["job"] == 2 and rows[0]["deck_a"] == "deck666" and rows[0]["seats"] == ["net", "net"]
    assert rows[1]["why"] == "over its time budget" and rows[1]["moves"] == 50


class CrawlingGame:
    """A game whose every move is slow and never ends (a board that keeps growing), or one that ends at once."""

    def __init__(self, crawl):
        self.crawl, self.turn, self.n = crawl, 1, 0

    def is_over(self):
        return not self.crawl and self.n >= 3

    def result(self):
        return "p0"

    def to_act(self):
        return self.n % 2

    def observation(self, who):
        field = [{"controller": 0, "card": 0}] * (self.n + 1) + [{"controller": 1, "card": 1}]
        return {"legal_actions": [("PassPriority", -1, -1, -1)] * 2, "battlefield": field}

    def baseline(self):
        return 0

    def step(self, i):
        self.n += 1
        if self.crawl:
            time.sleep(0.05)


def test_a_game_over_its_time_budget_is_a_timeout_with_no_samples(monkeypatch):
    import bolt.play_runner as pr

    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: CrawlingGame(crawl=s == 666))
    monkeypatch.setattr(pr, "encode", lambda obs, who: {"act_kind": [0, 0]})
    jobs = [("a", "b", 1, 0), ("a", "b", 666, 0), ("a", "b", 3, 0)]
    t = time.time()
    out = pr.run_games(None, jobs, seats=("rules", "rules"), record=(0, 1), game_s=0.5, max_moves=10**6)
    assert time.time() - t < 10
    assert [r["result"] for r in out["results"]] == ["p0", "timeout", "p0"]
    slow = out["results"][1]["slow"]
    assert slow["moves"] >= 5 and slow["permanents"][0] > slow["permanents"][1] == 1 and slow["most_common"]
    assert out["steps"] and all(st["game"] != 1 for st in out["steps"]), "the slow game leaves no samples"


def test_stop_ends_a_run_early_and_kills_its_workers():
    import pytest

    from bolt.play_runner import Stopped

    jobs = [("a", "b", 666, 0)] * 2  # both chunks would hang forever
    t, asked = time.time(), []

    def stop():
        asked.append(1)
        return len(asked) > 2

    with pytest.raises(Stopped):
        run_parallel(None, jobs, workers=2, chunk=1, hang_s=0, worker=worker, stop=stop, seats=("net", "net"))
    assert time.time() - t < 30


class SmartGame(CrawlingGame):
    """Ends after 16 moves. The smart seat's every move takes 0.04 s (its 50 ms cap); a rules move is instant,
    unless `crawl`, when the rules seat's steps take 0.05 s."""

    def __init__(self, crawl=False):
        super().__init__(crawl=False)
        self.slow_rules = crawl

    def is_over(self):
        return self.n >= 16

    def smart(self, rollouts, candidates, seed, ms=None):
        time.sleep(0.04)
        return 0

    def step(self, i):
        if self.slow_rules and self.n % 2 == 0:
            time.sleep(0.05)
        self.n += 1


def test_smart_moves_get_their_cap_on_top_of_the_game_budget(monkeypatch):
    import bolt.play_runner as pr

    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: SmartGame(crawl=s == 666))
    # 8 smart moves, 0.32 s: over a 0.2 s budget alone, inside it with 8 x 50 ms of allowance
    out = pr.run_games(None, [("a", "b", 1, 0)], seats=("rules", "smart"), record=(), smart=(1, 2, 50), game_s=0.2)
    assert out["results"][0]["result"] == "p0"
    # the allowance covers smart moves only: slow rules moves (8 x 0.05 s) still run the game out of budget
    out = pr.run_games(None, [("a", "b", 666, 0)], seats=("rules", "smart"), record=(), smart=(1, 2, 50), game_s=0.2)
    assert out["results"][0]["result"] == "timeout" and out["results"][0]["slow"]


class PanickingGame(CrawlingGame):
    """An engine whose 6th move panics, like finish_cast's .expect("payable")."""

    def __init__(self):
        super().__init__(crawl=False)

    def is_over(self):
        return False

    def legal_actions(self):
        return [("Cast", 7, -1, -1), ("PassPriority", -1, -1, -1)]

    def step(self, i):
        if self.n == 5:
            raise RuntimeError("payable")
        self.n += 1


def test_a_panicking_game_records_its_last_moves_and_message(monkeypatch):
    import bolt.play_runner as pr

    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: PanickingGame())
    out = pr.run_games(None, [("a", "b", 1, 0)], seats=("rules", "rules"), record=())
    r = out["results"][0]
    assert r["result"] == "error"
    p = r["panic"]
    # 5 moves made; the 6th, the one that panicked, is the last in the list
    assert p["message"] == "payable" and p["moves"] == 5 and [k for k, _ in p["last"]] == ["Cast"] * 6


def test_panicked_games_reach_the_hang_log(tmp_path):
    log = tmp_path / "hangs.jsonl"
    out = run_parallel(None, [("x", "y", 888, 1)], workers=1, hang_s=0, hang_log=str(log), worker=worker,
                       seats=("net", "rules"))  # fmt: skip
    assert out["results"][0]["result"] == "error"
    (row,) = [json.loads(x) for x in log.read_text().splitlines()]
    assert row["why"] == "engine panic" and row["seed"] == 888 and row["first"] == 1 and row["deck_a"] == "x"
    assert row["message"] == "payable" and row["moves"] == 12 and row["last"] == [["Cast", "Sol Ring"]]


def test_a_game_ended_by_an_engine_error_leaves_no_samples(monkeypatch):
    """2026-11-27: iteration 30's one panicked game, then a non-finite loss: its partial trajectory has no real
    outcome, so none of it reaches the update."""
    import bolt.play_runner as pr

    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: PanickingGame() if s == 666 else CrawlingGame(False))
    monkeypatch.setattr(pr, "encode", lambda obs, who: {"act_kind": [0, 0]})
    jobs = [("a", "b", 1, 0), ("a", "b", 666, 0)]
    out = pr.run_games(None, jobs, seats=("rules", "rules"), record=(0, 1), game_s=0)
    assert [r["result"] for r in out["results"]] == ["p0", "error"]
    assert out["steps"] and {st["game"] for st in out["steps"]} == {0}


def test_a_batch_with_an_errored_game_gives_a_finite_loss(monkeypatch):
    """The iteration-30 shape: one game panics mid-trajectory; the update over the batch stays finite."""
    import numpy as np
    import torch
    from test_play_commander import obs

    import bolt.play_runner as pr
    from bolt.play_data import to_arrays
    from bolt.play_encoding import encode as real_encode
    from bolt.play_net import PlayNet
    from bolt.play_ppo import gae, ppo_step

    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: ())
    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: PanickingGame() if s == 666 else CrawlingGame(False))
    monkeypatch.setattr(pr, "encode", lambda o, who: real_encode(obs(), who))
    jobs = [("a", "b", 1, 0), ("a", "b", 666, 0), ("a", "b", 3, 1)]
    out = pr.run_games(None, jobs, seats=("rules", "rules"), record=(0, 1), game_s=0)
    assert [r["result"] for r in out["results"]] == ["p0", "error", "p0"]
    assert out["results"][1]["dropped_steps"] > 0
    for st in out["steps"]:
        st["logp"], st["value"] = -0.7, 0.0
    arr = to_arrays(out["steps"], out["results"])
    adv, ret = gae(arr, lam=0.95)
    net = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    idx = np.arange(len(arr["action"]))
    st = ppo_step(net, None, torch.optim.AdamW(net.parameters(), lr=1e-3), arr, idx, torch.as_tensor(adv),
                  ret=torch.as_tensor(ret))  # fmt: skip
    assert np.isfinite(st["loss"]) and st["bad_groups"] == 0 and st["bad_grad"] == 0


@pytest.mark.parametrize("seats", [("net", "net"), ("net", "opp"), ("opp", "net")])
def test_an_errored_game_drops_its_recorded_steps_in_every_seat_layout(monkeypatch, seats):
    """2026-11-27: the farm's iteration-30 panic was an opp/net game (the past checkpoint in seat 0)."""
    from test_play_commander import obs

    import bolt.play_runner as pr
    from bolt.play_encoding import encode as real_encode
    from bolt.play_net import PlayNet

    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: ())
    monkeypatch.setattr(pr, "new_game", lambda a, b, s, f: PanickingGame() if s == 666 else CrawlingGame(False))
    monkeypatch.setattr(pr, "encode", lambda o, who: real_encode(obs(), who))
    net, opp = (PlayNet(n_cards=8, dim=16, layers=1, heads=2).eval() for _ in range(2))
    record = tuple(i for i, s in enumerate(seats) if s == "net")
    # progress on, as in the workers: its "longest game" line once shadowed the dropped-games set (2026-11-27)
    out = pr.run_games(net, [("a", "b", 666, 0), ("a", "b", 1, 0)], seats=seats, record=record, opp_net=opp,
                       game_s=0, progress=1)  # fmt: skip
    assert [r["result"] for r in out["results"]] == ["error", "p0"]
    assert out["results"][0]["dropped_steps"] == 3 * len(record), "the recorded seats' moves before the panic"
    assert {st["game"] for st in out["steps"]} == {1}
    assert {st["seat"] for st in out["steps"]} == set(record)
