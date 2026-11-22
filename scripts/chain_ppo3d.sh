#!/usr/bin/env bash
# 2026-11-21 night: resume ppo3 a third time (still creeping up at ~490) for 10 hours with the same
# settings, then test its best: 1,000 games against the rules-based and smart 1 x 2 players, then
# 1,000 games against smart 4 x 6 (the P3 goal) with the 4 x 6 vs rules-based control.
# One job at a time, 3 GB cap each, every iteration kept.
set -uo pipefail
cd "$(dirname "$0")/.."
echo "== ppo (resumed, night 2) $(date +%T)"
scripts/guard.sh 36000 3000 -- .venv/bin/python scripts/train_ppo_play.py --name ppo3 --resume \
  --init checkpoints/play/ppo2/iter_20.pt --lam 0.95 --games 640 --lr 1e-5 --kl 0.05 --kl-final 0.03 --kl-iters 100
echo "== training stopped (exit $?) $(date +%T)"
echo "== eval best, smart 1x2 $(date +%T)"
scripts/guard.sh 3600 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/ppo3/best.pt \
  --pairs 500 --workers 4 --no-control --name play_eval_ppo3d_best
echo "== eval best, smart 4x6 $(date +%T)"
scripts/guard.sh 10800 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/ppo3/best.pt \
  --pairs 500 --workers 4 --opponents smart --smart 4 6 --name play_eval_ppo3d_best_smart46
echo "== done $(date +%T)"
