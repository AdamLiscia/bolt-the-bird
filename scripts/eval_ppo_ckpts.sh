#!/usr/bin/env bash
# Re-test PPO checkpoints with 1,000 games against each fixed player (the same 500 unseen pairs as
# every other P3 evaluation), one after another: one heavy job at a time, 3 GB cap each.
set -euo pipefail
cd "$(dirname "$0")/.."
name=$1; shift
for it in "$@"; do
  echo "== $it $(date +%T)"
  scripts/guard.sh 3600 3000 -- .venv/bin/python scripts/eval_play.py --ckpt checkpoints/play/$name/iter_$it.pt \
    --pairs 500 --workers 4 --no-control --name play_eval_${name}_i$it
done
echo "== done $(date +%T)"
