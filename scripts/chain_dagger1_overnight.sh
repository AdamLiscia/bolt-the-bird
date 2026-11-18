#!/usr/bin/env bash
# Backup for chain_dagger1.sh (2026-11-18 evening, when the laptop slowed past its 5-hour training limit):
# waits for that chain to exit; if the evaluation never ran, resumes training from the last checkpoint
# under a 12-hour limit and evaluates. Does nothing if the main chain finished. One job at a time.
set -euo pipefail
cd "$(dirname "$0")/.."
while pgrep -f "bash scripts/chain_dagger1.sh" >/dev/null; do sleep 30; done
if [ -f results/play_eval_distill2.json ]; then echo "== main chain finished; nothing to do $(date +%T)"; exit 0; fi
echo "== train (backup, resumed) $(date +%T)"
scripts/guard.sh 43200 3000 -- .venv/bin/python scripts/train_distill.py --name distill2 --resume \
  --init checkpoints/play/distill1/best.pt --data data/distill data/dagger1 --epochs 1 --lr 1.5e-4 \
  --device cpu --threads 4
echo "== eval $(date +%T)"
scripts/guard.sh 7200 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/distill2/best.pt \
  --pairs 500 --workers 4 --name play_eval_distill2
echo "== done $(date +%T)"
