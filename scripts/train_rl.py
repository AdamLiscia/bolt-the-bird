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

    python scripts/train_rl.py                                   # start from the human model
    python scripts/train_rl.py --resume checkpoints/bot_rl.pt --name rl_long   # keep going
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
from bolt.judge import DeckJudge
from bolt.models import DraftTransformer
from bolt.rl import masked_kl
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--init",
        default=str(CK / "transformer.pt"),
        help="the human-imitation model: the KL anchor, the 7 opponents, and the eval baseline",
    )
    ap.add_argument("--resume", default="", help="start the bot from this checkpoint instead of from --init")
    ap.add_argument("--name", default="rl", help="writes checkpoints/bot_<name>.pt and results/<name>.json")
    ap.add_argument("--iters", type=int, default=150)
    ap.add_argument("--tables", type=int, default=128)
    ap.add_argument("--lr", type=float, default=3e-5)
    ap.add_argument("--kl", type=float, default=0.05)
    ap.add_argument("--eval-every", type=int, default=15)
    ap.add_argument("--eval-tables", type=int, default=512)
    ap.add_argument("--device", default="auto", help="cpu, mps, or auto. Smoke-test new code on cpu first.")
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
    gen = torch.Generator().manual_seed(1)
    history = []

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

    do_eval(0)
    t0 = time.time()
    for it in range(1, a.iters + 1):
        bot.eval()
        pools, log = run_tables(train_packs, a.tables, bot, ref, device, gen)
        counts = pools_to_counts(pools[:, 0], cards.n)
        _, reward, _ = build_decks(counts, judge_a, cards, device)
        adv = (reward - reward.mean()) / (reward.std() + 1e-6)
        adv_rows = torch.as_tensor(np.repeat(adv, 42), dtype=torch.float32, device=device)

        n = len(log["action"])
        perm = torch.randperm(n, device=device)
        kl_sum, agree = 0.0, 0.0
        opt.zero_grad(set_to_none=True)
        for s in range(0, n, 2048):
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
        }
        history.append(row)
        print(
            f"it {it:>3}  reward {row['reward']:.4f}  KL {row['kl']:.4f}  "
            f"agrees with human model {row['agree_with_human_model']:.1%}  {row['seconds']}s",
            flush=True,
        )
        if it % a.eval_every == 0 or it == a.iters:
            do_eval(it)
            torch.save(bot.state_dict(), CK / f"bot_{a.name}.pt")
            (DATA.parent / "results" / f"{a.name}.json").write_text(
                json.dumps({"args": vars(a), "history": history}, indent=1)
            )


if __name__ == "__main__":
    main()
