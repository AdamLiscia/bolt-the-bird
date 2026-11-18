#!/usr/bin/env bash
# 2026-11-18 evening: the laptop slowed to ~0.6 steps/s, past chain_dagger1.sh's 5-hour training limit.
# At the step-2,000 checkpoint, stop that chain and resume training under a 12-hour limit, then evaluate.
# One job at a time; the same 3 GB cap.
set -euo pipefail
cd "$(dirname "$0")/.."
until [ -f checkpoints/play/distill2/step_2000.pt ]; do sleep 20; done
sleep 30  # let the checkpoint finish writing and grading
pkill -f "scripts/chain_dagger1.sh" || true
pkill -f "train_distill.py --name distill2" || true
sleep 5
echo "== train (overnight) $(date +%T)"
scripts/guard.sh 43200 3000 -- .venv/bin/python scripts/train_distill.py --name distill2 --resume \
  --init checkpoints/play/distill1/best.pt --data data/distill data/dagger1 --epochs 1 --lr 1.5e-4 \
  --device cpu --threads 4
echo "== eval $(date +%T)"
scripts/guard.sh 7200 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/distill2/best.pt \
  --pairs 500 --workers 4 --name play_eval_distill2
echo "== done $(date +%T)"
