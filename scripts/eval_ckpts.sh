#!/usr/bin/env bash
# Evaluate several checkpoints, one after another (one heavy job at a time, 3 GB cap each).
set -euo pipefail
cd "$(dirname "$0")/.."
name=$1; shift
for step in "$@"; do
  echo "== $step $(date +%T)"
  scripts/guard.sh 3600 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/$name/step_$step.pt \
    --pairs 500 --workers 4 --no-control --name play_eval_${name}_s$step
done
echo "== done $(date +%T)"
