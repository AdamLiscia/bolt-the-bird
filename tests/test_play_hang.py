"""run_parallel's watchdog: a worker stuck in one engine call is killed, and its chunk rerun without that game."""

import json
import time

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
