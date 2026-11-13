"""Teach the draft bot to win instead of to copy: reinforcement learning against the judge.

Each iteration:
  1. Draft T simulated tables. Seat 0 is the learning bot; the other 7 seats are the frozen
     human-imitation model.
  2. Build seat 0's deck and score it with judge A. That score is the reward.
  3. Picks from drafts that scored above the batch average become more likely, picks from
     below-average drafts less likely (REINFORCE). A KL penalty keeps the bot close to the
     human model, so it improves on human drafting instead of wandering off into nonsense.

Evaluation never uses judge A: decks are built and scored by judge B, trained on a disjoint half
of the games. If the bot only learned to fool judge A, judge B won't agree.

With --reward played, step 2's reward is a played win rate instead: all 8 decks at the table are
built, and seat 0's deck plays each tablemate --games times in the Rust engine with the smart
player (bolt.played). The judge rewards decks that only beat players who never look ahead
(docs/engine/VALIDATION.md); this reward comes from players who do. A played evaluation then runs
every --played-eval-every iterations: the bot's and the human model's decks from the same test
tables each play the same field opponents with the same seeds.

    python scripts/train_rl.py                                   # start from the human model
    python scripts/train_rl.py --resume checkpoints/bot_rl.pt --name rl_long   # keep going
    python scripts/train_rl.py --reward played --device cpu --name rl_played --iters 300
    python scripts/train_rl.py --reward played --device cpu --resume checkpoints/bot_rl_played_it135.pt \\
        --start-iter 136 --iters 250 --name rl_played_b                    # pick a stopped run back up
"""

import argparse
import copy
import json
import time

import numpy as np
import torch

from bolt.cards import CardInfo
from bolt.data import DATA
from bolt.deckbuild import build_decks
from bolt.engine_worker import SafePlayer
from bolt.judge import DeckJudge
from bolt.models import DraftTransformer
from bolt.played import deck_lists, played_rewards, score
from bolt.rl import masked_kl, ppo_loss
from bolt.sim import opening_packs, pools_to_counts, run_tables

CK = DATA.parent / "checkpoints"
torch.backends.mha.set_fastpath_enabled(False)  # the fused inference path hangs on MPS after a training step


def load_judge(name, n, device):
    j = DeckJudge(n).to(device)
    j.load_state_dict(torch.load(CK / name, map_location=device))
    return j.eval()


@torch.no_grad()
def evaluate(bot, ref, packs, judge, cards, device, n_tables, seed):
    """Seat 0 drafts greedily with `bot` and then with `ref` on the same packs; judge B scores both."""
    out = {}
    for name, model in (("bot", bot), ("human_model", ref)):
        g = torch.Generator().manual_seed(seed)
        pools, _ = run_tables(packs, n_tables, model, ref, device, g, seat0_greedy=True)
        counts = pools_to_counts(pools[:, 0], cards.n)
        _, wr, _ = build_decks(counts, judge, cards, device)
        late = pools[:, 0, 14:].long() - 1
        out[name] = {"wr": wr, "rares": float(cards.rare[late.cpu().numpy()].sum(1).mean())}
    d = out["bot"]["wr"] - out["human_model"]["wr"]
    return {
        "bot_wr": float(out["bot"]["wr"].mean()),
        "human_model_wr": float(out["human_model"]["wr"].mean()),
        "gain": float(d.mean()),
        "gain_se": float(d.std() / np.sqrt(len(d))),
        "bot_late_rares": out["bot"]["rares"],
        "human_model_late_rares": out["human_model"]["rares"],
    }


FIELD, FIELD_GAMES = 64, 16


def played_field(cards, n_tables):
    """Field A (M6's 64 human-model decks), and each eval table's 16 opponents and seeds."""
    d = json.loads((DATA / "engine_decks_512.json").read_text())
    field = np.random.default_rng(0).choice(512, size=FIELD, replace=False)
    rng = np.random.default_rng(3)
    opps = [rng.choice([f for f in field if f != t], size=FIELD_GAMES, replace=False) for t in range(n_tables)]
    seeds = rng.integers(2**62, size=(n_tables, FIELD_GAMES))
    return d["human_model"]["decks"], opps, seeds


@torch.no_grad()
def played_side(model, ref, packs, judge, cards, device, n_tables, field, a, play):
    """One side's played score per eval table: its deck vs the table's field opponents."""
    decks, opps, seeds = field
    g = torch.Generator().manual_seed(12345)
    pools, _ = run_tables(packs, n_tables, model, ref, device, g, seat0_greedy=True)
    built, _, _ = build_decks(pools_to_counts(pools[:, 0], cards.n), judge, cards, device)
    mine = deck_lists(built)
    jobs, owner = [], []
    for t in range(n_tables):
        for k, f in enumerate(opps[t]):
            jobs.append((mine[t], decks[int(f)], int(seeds[t, k]), k % 2))
            owner.append(t)
    res = play(jobs, pilot="smart", rollouts=a.rollouts, candidates=a.candidates, threads=a.threads)
    return score(res, np.array(owner), n_tables)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--init",
        default=str(CK / "transformer.pt"),
        help="the human-imitation model: the KL anchor, the 7 opponents, and the eval baseline",
    )
    ap.add_argument("--resume", default="", help="start the bot from this checkpoint instead of from --init")
    ap.add_argument("--start-iter", type=int, default=1,
                    help="number iterations from here (a resumed run): fresh packs and game seeds, evals on schedule")  # fmt: skip
    ap.add_argument("--name", default="rl", help="writes checkpoints/bot_<name>.pt and results/<name>.json")
    ap.add_argument("--iters", type=int, default=150)
    ap.add_argument("--tables", type=int, default=128)
    ap.add_argument("--lr", type=float, default=3e-5)
    ap.add_argument("--kl", type=float, default=0.05)
    ap.add_argument(
        "--algo",
        choices=["reinforce", "ppo"],
        default="reinforce",
        help="reinforce: one step per batch of drafts; ppo: several clipped steps on the same drafts",
    )
    ap.add_argument("--ppo-epochs", type=int, default=4, help="ppo: passes over each batch of drafts")
    ap.add_argument("--ppo-clip", type=float, default=0.2, help="ppo: how far one batch may move a pick's probability")
    ap.add_argument("--ppo-minibatch", type=int, default=1024, help="ppo: picks per step")
    ap.add_argument("--eval-every", type=int, default=15)
    ap.add_argument("--eval-tables", type=int, default=512)
    ap.add_argument("--device", default="auto", help="cpu, mps, or auto. Smoke-test new code on cpu first.")
    ap.add_argument("--reward", choices=["judge", "played"], default="judge")
    ap.add_argument("--games", type=int, default=2, help="played reward: games against each tablemate")
    ap.add_argument("--rollouts", type=int, default=1, help="smart player: playouts per option")
    ap.add_argument("--candidates", type=int, default=2, help="smart player: options considered")
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--played-eval-every", type=int, default=30)
    ap.add_argument("--played-eval-tables", type=int, default=256)
    a = ap.parse_args()

    if a.device == "auto":
        a.device = "mps" if torch.backends.mps.is_available() else "cpu"
    device = torch.device(a.device)
    torch.manual_seed(0)
    cards = CardInfo()
    ref = DraftTransformer(cards.n).to(device)
    ref.load_state_dict(torch.load(a.init, map_location=device))
    ref.eval()
    bot = copy.deepcopy(ref)
    if a.resume:
        bot.load_state_dict(torch.load(a.resume, map_location=device))
        print(f"resuming bot from {a.resume}", flush=True)
    judge_a = load_judge("judge_h0.pt", cards.n, device)
    judge_b = load_judge("judge_h1.pt", cards.n, device)
    train_packs, test_packs = opening_packs("train"), opening_packs("test")
    opt = torch.optim.Adam(bot.parameters(), lr=a.lr)
    gen = torch.Generator().manual_seed(1 if a.start_iter == 1 else 1_000 + a.start_iter)
    history = []

    field = played_field(cards, a.played_eval_tables) if a.reward == "played" else None
    # Engine games run in a separate process, restarted if it crashes (bolt.engine_worker).
    player = SafePlayer() if a.reward == "played" else None
    ref_played = None

    def do_played_eval(it):
        nonlocal ref_played
        bot.eval()
        args = (test_packs, judge_b, cards, device, a.played_eval_tables, field, a, player.play_games)
        if ref_played is None:
            ref_played = played_side(ref, ref, *args)[0]
        mine, errors = played_side(bot, ref, *args)
        d = mine - ref_played
        r = {
            "iter": it,
            "played_gain": float(np.nanmean(d)),
            "played_gain_se": float(np.nanstd(d) / np.sqrt(np.isfinite(d).sum())),
            "errors": errors,
        }
        history.append({"played_eval": r})
        print(
            f"  PLAYED EVAL it {it}: gain {r['played_gain'] * 100:+.2f} ± {r['played_gain_se'] * 100:.2f} pts "
            f"(smart {a.rollouts} x {a.candidates}, {a.played_eval_tables} tables x {FIELD_GAMES} field games)",
            flush=True,
        )

    def do_eval(it):
        bot.eval()
        r = evaluate(bot, ref, test_packs, judge_b, cards, device, a.eval_tables, seed=12345)
        r["iter"] = it
        history.append({"eval": r})
        print(
            f"  EVAL it {it}: bot {r['bot_wr']:.2%} vs human model {r['human_model_wr']:.2%}  "
            f"gain {r['gain'] * 100:+.2f} ± {r['gain_se'] * 100:.2f} pts (judge B)  "
            f"rares taken after pack 1: {r['bot_late_rares']:.2f} vs {r['human_model_late_rares']:.2f}",
            flush=True,
        )

    do_eval(a.start_iter - 1)
    if a.reward == "played":
        do_played_eval(a.start_iter - 1)
    t0 = time.time()
    for it in range(a.start_iter, a.iters + 1):
        bot.eval()
        pools, log = run_tables(train_packs, a.tables, bot, ref, device, gen)
        errors = 0
        if a.reward == "played":
            # Every seat's deck, built the usual way; seat 0 plays the other seven.
            built, _, _ = build_decks(pools_to_counts(pools.reshape(-1, 42), cards.n), judge_a, cards, device)
            lists = deck_lists(built)
            decks = [lists[8 * t : 8 * t + 8] for t in range(a.tables)]
            reward, errors = played_rewards(
                decks, a.games, 10_000_000 * it, a.rollouts, a.candidates, a.threads, play=player.play_games
            )
            reward = np.nan_to_num(reward, nan=float(np.nanmean(reward)))
        else:
            counts = pools_to_counts(pools[:, 0], cards.n)
            _, reward, _ = build_decks(counts, judge_a, cards, device)
        adv = (reward - reward.mean()) / (reward.std() + 1e-6)
        adv_rows = torch.as_tensor(np.repeat(adv, 42), dtype=torch.float32, device=device)

        n = len(log["action"])

        def state(idx, log=log):
            return (log["pool"][idx], log["pack"][idx], log["pack_number"][idx], log["pick_number"][idx])

        if a.algo == "ppo":
            # The chosen cards' log-probabilities when these drafts were played, then several clipped
            # steps on the same drafts (bolt.rl.ppo_loss).
            with torch.no_grad():
                old_lp = torch.cat([
                    torch.log_softmax(bot(*state(torch.arange(s, min(s + 2048, n), device=device))), 1)
                    .gather(1, log["action"][s : s + 2048].long()[:, None]).squeeze(1)
                    for s in range(0, n, 2048)
                ])  # fmt: skip
            clipped, steps = 0.0, 0
            for _ in range(a.ppo_epochs):
                perm = torch.randperm(n, device=device)
                for s in range(0, n, a.ppo_minibatch):
                    idx = perm[s : s + a.ppo_minibatch]
                    logits = bot(*state(idx))
                    with torch.no_grad():
                        ref_logits = ref(*state(idx))
                    loss, st = ppo_loss(logits, ref_logits, log["action"][idx].long(), old_lp[idx], adv_rows[idx],
                                        a.ppo_clip, a.kl)  # fmt: skip
                    opt.zero_grad(set_to_none=True)
                    loss.backward()
                    gnorm = torch.nn.utils.clip_grad_norm_(bot.parameters(), 1.0)
                    if not torch.isfinite(gnorm):
                        raise RuntimeError(f"non-finite gradient at iteration {it}")
                    opt.step()
                    clipped, steps = clipped + st["clipped"], steps + 1
            if not all(torch.isfinite(p).all() for p in bot.parameters()):
                raise RuntimeError(f"bot weights went non-finite at iteration {it}")
            with torch.no_grad():  # KL and agreement after the update, for the log
                kl_sum, agree = 0.0, 0.0
                for s in range(0, n, 2048):
                    idx = torch.arange(s, min(s + 2048, n), device=device)
                    logits, ref_logits = bot(*state(idx)), ref(*state(idx))
                    kl_sum += masked_kl(logits, ref_logits).sum().item()
                    agree += (logits.argmax(1) == ref_logits.argmax(1)).sum().item()
            ppo_clipped = clipped / max(1, steps)
        perm = torch.randperm(n, device=device)
        if a.algo == "reinforce":
            kl_sum, agree = 0.0, 0.0
            opt.zero_grad(set_to_none=True)
            ppo_clipped = None
        for s in range(0, n, 2048) if a.algo == "reinforce" else ():
            idx = perm[s : s + 2048]
            args = (log["pool"][idx], log["pack"][idx], log["pack_number"][idx], log["pick_number"][idx])
            logits = bot(*args)
            with torch.no_grad():
                ref_logits = ref(*args)
            act = log["action"][idx].long()
            lp = torch.log_softmax(logits, 1).gather(1, act[:, None]).squeeze(1)  # chosen card is always in the pack
            kl = masked_kl(logits, ref_logits)
            loss = (-(adv_rows[idx] * lp).sum() + a.kl * kl.sum()) / n
            loss.backward()
            kl_sum += kl.sum().item()
            agree += (logits.argmax(1) == ref_logits.argmax(1)).sum().item()
        if a.algo == "reinforce":
            gnorm = torch.nn.utils.clip_grad_norm_(bot.parameters(), 1.0)
            if not torch.isfinite(gnorm):
                raise RuntimeError(f"non-finite gradient at iteration {it}")
            opt.step()
            if not all(torch.isfinite(p).all() for p in bot.parameters()):
                raise RuntimeError(f"bot weights went non-finite at iteration {it}")
        row = {
            "iter": it,
            "reward": float(reward.mean()),
            "kl": kl_sum / n,
            "agree_with_human_model": agree / n,
            "seconds": round(time.time() - t0),
            "errors": errors,
            "engine_restarts": player.restarts if player else 0,
            "ppo_clipped": ppo_clipped,
        }
        history.append(row)
        print(
            f"it {it:>3}  reward {row['reward']:.4f}  KL {row['kl']:.4f}  "
            f"agrees with human model {row['agree_with_human_model']:.1%}  {row['seconds']}s",
            flush=True,
        )
        if a.reward == "played" and (it % a.played_eval_every == 0 or it == a.iters):
            do_played_eval(it)
        if it % a.eval_every == 0 or it == a.iters:
            do_eval(it)
            torch.save(bot.state_dict(), CK / f"bot_{a.name}.pt")
            (DATA.parent / "results" / f"{a.name}.json").write_text(
                json.dumps({"args": vars(a), "history": history}, indent=1)
            )


if __name__ == "__main__":
    main()
