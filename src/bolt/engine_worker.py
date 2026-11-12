"""Engine games in a separate process, restarted if it dies.

Three times a long run has died of memory corruption inside the engine (docs/engine/QUESTIONS.md),
twice in training runs that hold PyTorch in the same process. Whatever the cause, a crash here only
kills the worker: `SafePlayer` starts a fresh one and replays the batch, and since games are
deterministic, the replayed results are the ones the crashed batch would have given. The worker
imports the engine and nothing else (no PyTorch).

    with SafePlayer() as p:
        results = p.play_games(jobs, pilot="smart", rollouts=1, candidates=2, threads=8)
"""

import multiprocessing as mp
import sys
from concurrent.futures import ProcessPoolExecutor
from concurrent.futures.process import BrokenProcessPool


def _play(jobs, **kw):
    import bolt_engine as be

    return be.play_games(jobs, **kw)


class SafePlayer:
    def __init__(self, target=_play, target_args=(), retries: int = 3):
        """`target(*target_args, jobs, **kw)` runs in the worker; tests pass one that crashes."""
        self.target, self.target_args, self.retries = target, target_args, retries
        self.restarts = 0
        self._pool = None

    def _start(self):
        self._pool = ProcessPoolExecutor(max_workers=1, mp_context=mp.get_context("spawn"))

    def play_games(self, jobs, **kw):
        for attempt in range(self.retries + 1):
            if self._pool is None:
                self._start()
            try:
                return self._pool.submit(self.target, *self.target_args, jobs, **kw).result()
            except BrokenProcessPool:
                self._pool.shutdown(wait=False, cancel_futures=True)
                self._pool = None
                if attempt == self.retries:
                    break
                self.restarts += 1
                print(
                    f"engine worker died; restart {self.restarts}, replaying {len(jobs)} games",
                    file=sys.stderr,
                    flush=True,
                )
        raise RuntimeError(
            f"engine worker died {self.retries + 1} times in a row on one batch; gave up after {self.retries} restarts"
        )

    def close(self):
        if self._pool is not None:
            self._pool.shutdown(wait=True)
            self._pool = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
