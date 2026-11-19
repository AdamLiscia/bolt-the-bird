#!/usr/bin/env bash
# P3d first run: self-play PPO from the distilled network, overnight on the laptop.
# One job, 4 workers + 4 training threads, 3 GB cap, 12-hour limit; every iteration kept; resumable.
set -euo pipefail
cd "$(dirname "$0")/.."
echo "== ppo $(date +%T)"
scripts/guard.sh 43200 3000 -- .venv/bin/python scripts/train_ppo_play.py --name ppo1 --resume
echo "== done $(date +%T)"
