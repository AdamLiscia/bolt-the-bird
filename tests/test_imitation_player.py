"""The imitation player (Phase 4, P2): turn models choosing casts, attacks and blocks in the engine."""

import random

import pytest
import torch

be = pytest.importorskip("bolt_engine")

from bolt.imitation_player import ImitationPlayer, play_game, zones_of  # noqa: E402


class Const(torch.nn.Module):
    """A stand-in turn model that scores every slot the same."""

    def __init__(self, value: float):
        super().__init__()
        self.value = value

    def forward(self, zones, scalars, target):
        x = zones[target]
        return torch.full(x.shape, self.value).masked_fill(x == 0, float("-inf"))


def deck(seed):
    rng = random.Random(seed)
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)


def player(v):
    return ImitationPlayer(Const(v), Const(v), Const(v))


def test_zones_describe_the_position_from_one_side():
    g = be.Game(deck(1), deck(2), seed=3, first=0)
    for _ in range(40):
        if g.is_over():
            break
        g.step(g.baseline())
    z, s = zones_of(g.observation(0), me=0, first=0)
    obs = g.observation(0)
    assert sorted(int(c) - 1 for c in z["hand"][0] if c) == sorted(c for _, c in obs["players"][0]["hand"])
    mine = [e for e in obs["battlefield"] if e["controller"] == 0 and e["land"]]
    assert (z["my_lands"][0] > 0).sum() == len(mine)
    assert s.shape == (1, 5) and float(s[0, 4]) == 1.0, "on the play"


@pytest.mark.parametrize("value", [10.0, -10.0])
def test_games_finish_with_legal_moves_whatever_the_models_say(value):
    for seed in range(3):
        r = play_game(deck(seed), deck(seed + 50), seed=seed, first=seed % 2, players=[player(value), None])
        assert r in ("p0", "p1", "draw", "timeout")


def test_a_player_whose_models_say_no_never_casts_on_its_own_turn():
    p = player(-10.0)
    g = be.Game(deck(4), deck(5), seed=9, first=0)
    for _ in range(3000):
        if g.is_over():
            break
        who = g.to_act()
        obs = g.observation(who)
        i = p.choose(g, who, first=0) if who == 0 else g.baseline()
        kind = obs["legal_actions"][i][0]
        if (
            who == 0
            and obs["pending"][0] == "Priority"
            and obs["active"] == 0
            and obs["step"] in ("Main1", "Main2")
            and not obs["stack"]
        ):
            assert not kind.startswith("Cast") and kind != "PlayLand"
        g.step(i)


def test_identical_cards_pool_their_scores():
    """The cast model can't tell copies apart, so it splits its confidence across them: three Plains
    at 40% each still mean "play a Plains". One copy of a spell at 40% stays in hand."""
    p = ImitationPlayer(Const(-0.405), Const(-10.0), Const(-10.0))  # every card scores 0.40
    g = be.Game(deck(6), deck(7), seed=2, first=0)
    played = None
    for _ in range(400):
        who = g.to_act()
        obs = g.observation(who)
        if who == 0 and obs["pending"][0] == "Priority" and obs["active"] == 0 and obs["step"] == "Main1":
            hand = [c for _, c in obs["players"][0]["hand"]]
            lands_in_hand = sum(be.card_names()[c] in ("Plains", "Mountain") for c in hand)
            a = obs["legal_actions"][p.choose(g, 0, first=0)]
            if lands_in_hand >= 2:
                played = a[0]
                break
        g.step(g.baseline() if who == 1 else p.choose(g, who, first=0))
    assert played == "PlayLand"


def test_a_second_game_does_not_reuse_the_first_games_plans():
    """Plans are kept per turn; a new game must not inherit the last game's (Python reuses object
    ids, so keying on id(game) alone let stale plans make the player pass every turn)."""
    from bolt.imitation_player import play_game

    p = ImitationPlayer(Const(5.0), Const(-10.0), Const(-10.0))  # wants to play everything
    play_game(deck(1), deck(2), seed=1, first=0, players=[p, None], max_turns=8)
    assert p.plans == {}, "cleared when the game ends"
    for seed in (2, 3):
        g = be.Game(deck(seed), deck(seed + 9), seed=seed, first=0)
        p.reset()
        while True:
            who = g.to_act()
            obs = g.observation(who)
            if who == 0 and obs["pending"][0] == "Priority" and obs["step"] == "Main1" and obs["active"] == 0:
                assert obs["legal_actions"][p.choose(g, 0, first=0)][0] == "PlayLand"
                break
            g.step(g.baseline())
