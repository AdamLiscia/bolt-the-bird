#!/usr/bin/env bash
# P3d second run (2026-11-19 night): from ppo1's best (iteration 190, 54.8% vs the rules-based player),
# leashed to it, with a tighter leash that never fully loosens (0.05 -> 0.03) and half the learning
# rate. ppo1 declined after its leash loosened to 0.005 at iteration 100.
# One job, 4 workers + 4 training threads, 3 GB cap, 12-hour limit; every iteration kept; resumable.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== ppo $(date +%T)"
scripts/guard.sh 43200 3000 -- .venv/bin/python scripts/train_ppo_play.py --name ppo2 --resume \
  --init checkpoints/play/ppo1/iter_190.pt --lr 1e-5 --kl 0.05 --kl-final 0.03 --kl-iters 100
echo "== done $(date +%T)"
