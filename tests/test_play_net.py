"""P3a: reading an engine position and its legal moves, and the network that scores the moves."""

import random

import pytest
import torch

be = pytest.importorskip("bolt_engine")

from bolt.play_encoding import FEATS, ZONE, collate, encode  # noqa: E402
from bolt.play_net import PlayNet  # noqa: E402


def deck(seed):
    rng = random.Random(seed)
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)


def positions(n=40, seed=0):
    """Real decision points from a rules-based game, (observation, player) pairs."""
    g = be.Game(deck(seed), deck(seed + 1), seed=seed, first=seed % 2)
    out = []
    while not g.is_over() and len(out) < n:
        who = g.to_act()
        obs = g.observation(who)
        if len(obs["legal_actions"]) > 1:
            out.append((obs, who))
        g.step(g.baseline())
    return out


def test_the_encoding_shows_only_what_the_player_can_see():
    for obs, me in positions(30, seed=1):
        e = encode(obs, me)
        mine = obs["players"][me]["hand"]
        hand_tokens = [i for i, z in enumerate(e["zone"]) if z == ZONE["my_hand"]]
        assert len(hand_tokens) == len(mine)
        assert not any(z == ZONE.get("opp_hand") for z in e["zone"]), "the opponent's hand is a count, not cards"
        assert e["globals"][e["global_names"].index("opp_hand")] == obs["players"][1 - me]["hand_size"] / 10


def test_every_move_points_at_the_object_it_names():
    for obs, me in positions(60, seed=2):
        e = encode(obs, me)
        for k, (_kind, a, b, c) in enumerate(obs["legal_actions"]):
            for slot, obj in enumerate((a, b, c)):
                ptr = e["act_ptr"][k][slot]
                if ptr >= 0:
                    assert e["obj_id"][ptr] == obj
            assert e["act_kind"][k] >= 0


def test_attackers_are_marked_during_combat():
    g = be.Game(deck(5), deck(6), seed=5)
    for _ in range(3000):
        if g.is_over():
            break
        who = g.to_act()
        obs = g.observation(who)
        if obs["combat"] and obs["combat"]["attackers"]:
            e = encode(obs, who)
            att = FEATS.index("attacking")
            flagged = {e["obj_id"][i] for i in range(len(e["obj_id"])) if e["feats"][i][att] == 1}
            assert flagged == set(obs["combat"]["attackers"])
            return
        g.step(g.baseline())
    pytest.fail("no combat in 3000 steps")


def net():
    torch.manual_seed(0)
    return PlayNet(len(be.card_names()), dim=32, layers=2, heads=4).eval()


def test_the_order_of_objects_does_not_matter():
    m = net()
    obs, me = positions(25, seed=3)[-1]
    e = encode(obs, me)
    n = len(e["obj_id"])
    perm = list(range(n))
    random.Random(0).shuffle(perm)
    inv = {old: new for new, old in enumerate(perm)}
    e2 = dict(e)
    for key in ("obj_id", "card", "zone", "feats"):
        e2[key] = [e[key][i] for i in perm]
    e2["act_ptr"] = [[inv[p] if p >= 0 else -1 for p in ptrs] for ptrs in e["act_ptr"]]
    a, va = m(collate([e]))
    b, vb = m(collate([e2]))
    assert torch.allclose(a, b, atol=1e-5) and torch.allclose(va, vb, atol=1e-5)


def test_a_batch_of_positions_gives_a_distribution_over_each_ones_legal_moves():
    m = net()
    ps = positions(40, seed=4)[::8]
    batch = collate([encode(o, me) for o, me in ps])
    logits, value = m(batch)
    assert logits.shape[0] == len(ps) and value.shape == (len(ps),)
    for i, (o, _) in enumerate(ps):
        n = len(o["legal_actions"])
        assert torch.isfinite(logits[i, :n]).all() and torch.isinf(logits[i, n:]).all()
        assert abs(torch.softmax(logits[i], 0).sum().item() - 1) < 1e-5
    assert ((value >= -1) & (value <= 1)).all()


def test_the_spell_being_targeted_for_is_in_the_input():
    """2026-11-23: while targets are chosen, the spell isn't on the stack yet; without it in the input
    the network couldn't tell Giant Growth from Stab."""
    obs, me = positions(30, seed=6)[-1]
    gg = be.index_of("Giant Growth")
    plain = encode({**obs, "choosing": None}, me)
    seen = encode(
        {**obs, "choosing": {"id": 999, "card": gg, "controller": me, "spell": True, "targets_chosen": 0}}, me
    )
    assert len(seen["card"]) == len(plain["card"]) + 1
    k = seen["obj_id"].index(999)
    assert seen["card"][k] == gg + 1 and seen["zone"][k] == ZONE["my_stack"]
    other = encode(
        {**obs, "choosing": {"id": 999, "card": gg, "controller": 1 - me, "spell": True, "targets_chosen": 0}}, me
    )
    assert other["zone"][other["obj_id"].index(999)] == ZONE["opp_stack"]
