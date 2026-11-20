#!/usr/bin/env bash
# P3d third run (2026-11-20, while Adam is at work): step-by-step credit (GAE, lambda 0.95) and twice
# the games per update (640), from ppo2 iteration 20 (56.1% / 48.5% on the 1,000-game re-test),
# leashed to it (KL 0.05 -> 0.03), lr 1e-5. Training stops after 8 hours; then best.pt (by the quick
# evaluations) gets the 1,000-game test. One job at a time, 3 GB cap, every iteration kept.
set -uo pipefail
cd "$(dirname "$0")/.."
echo "== ppo $(date +%T)"
scripts/guard.sh 28800 3000 -- .venv/bin/python scripts/train_ppo_play.py --name ppo3 --resume \
  --init checkpoints/play/ppo2/iter_20.pt --lam 0.95 --games 640 --lr 1e-5 --kl 0.05 --kl-final 0.03 --kl-iters 100
echo "== training stopped (exit $?) $(date +%T)"
if [ -f checkpoints/play/ppo3/best.pt ]; then
  echo "== eval best $(date +%T)"
  scripts/guard.sh 3600 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/ppo3/best.pt \
    --pairs 500 --workers 4 --no-control --name play_eval_ppo3_best
fi
echo "== done $(date +%T)"
