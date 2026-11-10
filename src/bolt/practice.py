"""Practice drafting: you take seat 0 at an 8-seat table, the other 7 seats are bots.

The table works like `bolt.sim.run_tables`: real opening packs, 3 rounds of 14 picks, passing left,
right, left, every seat picking at the same time. The bots sample their picks the way people make
them (the human-imitation model). A coach model (Bolt) scores every card in your pack at every
pick, and each pick is logged with what Bolt would have taken, for the review afterwards.
`redraft_with_coach` replays the same table from the same opening packs with Bolt in your seat.
"""

import numpy as np
import torch

SEATS, ROUNDS, PACK = 8, 3, 14


class PracticeDraft:
    def __init__(self, packs: np.ndarray, bots, coach, n_cards: int, seed: int):
        """packs: (n, 15) real opening packs (card ids + 1, zero-padded)."""
        self.source, self.bots, self.coach_model, self.n_cards, self.seed = packs, bots, coach, n_cards, seed
        rng = np.random.default_rng(seed)
        self.packs_by_round = np.stack([packs[rng.choice(len(packs), SEATS, replace=False)] for _ in range(ROUNDS)])
        self.gen = torch.Generator().manual_seed(seed)
        self.round, self.pick_number = 0, 0
        self.cur = torch.as_tensor(self.packs_by_round[0].copy())
        self.pool_t = torch.zeros(SEATS, ROUNDS * PACK, dtype=torch.long)
        self.log: list[dict] = []

    @property
    def done(self) -> bool:
        return self.round >= ROUNDS

    @property
    def position(self) -> int:
        return self.round * PACK + self.pick_number

    def human_pack(self) -> list[int]:
        """The cards in front of you (card ids), in pack order."""
        return [int(c) - 1 for c in self.cur[0] if c]

    def pools(self) -> list[list[int]]:
        """Every seat's picks so far (card ids), in pick order; seat 0 is you."""
        return [[int(c) - 1 for c in seat if c] for seat in self.pool_t]

    def _logits(self, model, seats: slice) -> torch.Tensor:
        n = len(range(*seats.indices(SEATS)))
        pn = torch.full((n,), self.round, dtype=torch.int8)
        pk = torch.full((n,), self.pick_number, dtype=torch.int8)
        with torch.no_grad():
            return model(self.pool_t[seats], self.cur[seats], pn, pk)

    def coach(self) -> list[float]:
        """Bolt's probability for each card in your pack, in pack order (copies share it)."""
        probs = torch.softmax(self._logits(self.coach_model, slice(0, 1))[0], 0)
        pack = self.human_pack()
        return [float(probs[c]) / pack.count(c) for c in pack]

    def pick(self, index: int) -> None:
        """Take the card at `index` of your pack; the bots pick at the same time, then packs pass."""
        pack = self.human_pack()
        if self.done or not 0 <= index < len(pack):
            raise ValueError(f"no card at position {index} in a pack of {len(pack)}")
        probs = self.coach()
        top = int(np.argmax(probs))
        took = pack[index]
        self.log.append({
            "position": self.position, "pack": pack, "took": took,
            "coach_top": pack[top], "coach_prob_took": probs[index] * pack.count(took),
            "coach_prob_top": probs[top] * pack.count(pack[top]),
        })  # fmt: skip
        logits = self._logits(self.bots, slice(1, SEATS))
        choice = torch.multinomial(torch.softmax(logits, 1), 1, generator=self.gen).squeeze(1)
        choice = torch.cat([torch.tensor([took]), choice])
        match = self.cur == (choice[:, None] + 1)
        first = match & (match.cumsum(-1) == 1)
        self.cur = self.cur.masked_fill(first, 0)
        self.pool_t[:, self.position] = choice + 1
        direction = 1 if self.round % 2 == 0 else -1
        self.cur = torch.roll(self.cur, direction, dims=0)
        self.pick_number += 1
        if self.pick_number == PACK:
            self.round, self.pick_number = self.round + 1, 0
            if not self.done:
                self.cur = torch.as_tensor(self.packs_by_round[self.round].copy())

    def redraft_with_coach(self) -> "PracticeDraft":
        """The same table from the same opening packs, with Bolt taking its top pick in your seat."""
        d = PracticeDraft(self.source, self.bots, self.coach_model, self.n_cards, self.seed)
        while not d.done:
            probs = d.coach()
            d.pick(int(np.argmax(probs)))
        return d


def check_deck(deck: list[int], pool: list[int], basics: set[int]) -> list[str]:
    """Problems with a deck built from `pool` (empty if it's fine): at least 40 cards, and every
    card you drafted at most as many times as you took it; basic lands are free."""
    problems = []
    if len(deck) < 40:
        problems.append(f"a deck needs at least 40 cards; this one has {len(deck)}")
    for c in sorted(set(deck) - basics):
        if deck.count(c) > pool.count(c):
            problems.append(f"card {c}: {deck.count(c)} in the deck but {pool.count(c)} drafted")
    return problems


def _score(result: str) -> float | None:
    return {"p0": 1.0, "p1": 0.0, "error": None}.get(result, 0.5)


def _rate(scores: list[float]) -> dict:
    n = len(scores)
    p = sum(scores) / n if n else 0.0
    half = 1.96 * (p * (1 - p) / n) ** 0.5 if n else 0.0
    return {"win_rate": p, "games": n, "low": max(0.0, p - half), "high": min(1.0, p + half)}


def grade(draft: PracticeDraft, deck: list[int], table: list[list[int]], bolt_deck: list[int],
          games: int = 20, play=None, **pilot) -> dict:  # fmt: skip
    """Your deck and Bolt's (drafted from your seat) each play the 7 decks at your table, `games`
    times each, with the same opponents and seeds, so the two win rates compare directly.
    `play` is `bolt_engine.play_games` (tests pass a stand-in); `pilot` goes to it.
    Also returns the pick-by-pick review against Bolt."""
    if play is None:
        import bolt_engine as be

        play = be.play_games
    pairs = [(o, k) for o in range(len(table)) for k in range(games)]
    seeds = [draft.seed * 100_003 + 1_000 * o + k for o, k in pairs]
    jobs = [(deck, table[o], s, k % 2) for (o, k), s in zip(pairs, seeds, strict=True)]
    jobs += [(bolt_deck, table[o], s, k % 2) for (o, k), s in zip(pairs, seeds, strict=True)]
    res = [_score(r) for r, _ in play(jobs, **pilot)]
    yours, bolts = res[: len(pairs)], res[len(pairs) :]
    both = [(a, b) for a, b in zip(yours, bolts, strict=True) if a is not None and b is not None]
    diff = [a - b for a, b in both]
    mean = sum(diff) / len(diff) if diff else 0.0
    sd = (sum((x - mean) ** 2 for x in diff) / max(1, len(diff) - 1)) ** 0.5
    per_opp = []
    for o in range(len(table)):
        mine = [x for (oo, _), x in zip(pairs, yours, strict=True) if oo == o and x is not None]
        per_opp.append(sum(mine) / len(mine) if mine else None)
    return {
        "you": _rate([x for x in yours if x is not None]),
        "bolt": _rate([x for x in bolts if x is not None]),
        "difference": {"points": 100 * mean, "se_points": 100 * sd / max(1, len(diff)) ** 0.5},
        "by_opponent": per_opp,
        "picks": draft.log,
        "agreement": sum(e["took"] == e["coach_top"] for e in draft.log) / max(1, len(draft.log)),
        "errors": sum(x is None for x in res),
    }
