"""A reward from played games: seat 0's deck against its 7 tablemates in the Rust engine.

The judge rewards deck traits that only pay off against players who never look ahead
(docs/engine/VALIDATION.md, the strength ladder). This reward comes from games played by the
smart player instead: each tablemate twice or more, alternating who goes first.
"""

import numpy as np


def table_jobs(decks, games: int, seed: int):
    """decks[t][s] is seat s's deck at table t. Returns engine jobs and the table each belongs to."""
    jobs, owner = [], []
    for t, table in enumerate(decks):
        for s in range(1, 8):
            for k in range(games):
                jobs.append((table[0], table[s], seed + 1000 * t + 10 * s + k, k % 2))
                owner.append(t)
    return jobs, np.array(owner)


def score(results, owner: np.ndarray, n_tables: int):
    """Seat 0's win rate per table (draws and timeouts count half); crashed games are left out."""
    s = np.array([{"p0": 1.0, "p1": 0.0, "error": np.nan}.get(r, 0.5) for r, _ in results])
    errors = int(np.isnan(s).sum())
    return np.array([np.nanmean(s[owner == t]) for t in range(n_tables)]), errors


def played_rewards(decks, games: int, seed: int, rollouts: int = 1, candidates: int = 2, threads: int = 8):
    """Seat 0's played win rate at each table, and how many games crashed."""
    import bolt_engine as be

    jobs, owner = table_jobs(decks, games, seed)
    res = be.play_games(jobs, pilot="smart", rollouts=rollouts, candidates=candidates, threads=threads)
    return score(res, owner, len(decks))


def deck_lists(counts: np.ndarray) -> list[list[int]]:
    """Card-count rows (N, n_cards) to the engine's card-number lists."""
    return [np.repeat(np.arange(counts.shape[1]), c.astype(int)).tolist() for c in counts]
