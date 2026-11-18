#!/usr/bin/env bash
# P3c, one job at a time (2026-11-18: two 6-worker jobs at once froze the laptop).
# Each step starts only after the previous one exits cleanly; each is capped at 3 GB and 4 workers.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== data $(date +%T)"
scripts/guard.sh 7200 3000 -- .venv/bin/python scripts/gen_distill.py --shards 40 --workers 4
echo "== train $(date +%T)"
scripts/guard.sh 14400 3000 -- .venv/bin/python scripts/train_distill.py --name distill1 --resume
echo "== eval $(date +%T)"
scripts/guard.sh 7200 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/distill1/best.pt \
  --pairs 500 --workers 4 --name play_eval_distill1
echo "== done $(date +%T)"
