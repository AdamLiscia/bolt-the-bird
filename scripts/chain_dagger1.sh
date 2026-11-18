#!/usr/bin/env bash
# P3c round 2 (DAgger), one job at a time, each capped at 3 GB, at most 4 workers.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== data $(date +%T)"
scripts/guard.sh 10800 3000 -- .venv/bin/python scripts/gen_distill.py --ckpt checkpoints/play/distill1/best.pt \
  --out data/dagger1 --shards 24 --workers 4
echo "== train $(date +%T)"
scripts/guard.sh 18000 3000 -- .venv/bin/python scripts/train_distill.py --name distill2 --resume \
  --init checkpoints/play/distill1/best.pt --data data/distill data/dagger1 --epochs 1 --lr 1.5e-4 \
  --device cpu --threads 4
echo "== eval $(date +%T)"
scripts/guard.sh 7200 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/distill2/best.pt \
  --pairs 500 --workers 4 --name play_eval_distill2
echo "== done $(date +%T)"
