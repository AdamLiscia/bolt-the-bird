#!/usr/bin/env bash
# Overnight 2026-11-24 → 25 (Adam asleep; Mac plugged in). Run under `caffeinate -i` so the Mac stays
# awake exactly as long as these jobs do. Two compute jobs side by side, both capped, plus a watchdog:
#   1. Tarkir drafter (transformer, 3 epochs) on the GPU, GPU allocator capped, 4 GB guard, per-epoch
#      checkpoints (checkpoints/TDM_transformer_epochN.pt).
#   2. Expert targeting: continue the learned player's PPO from ppo4 (it is still unlearning the pump
#      habit), 2 workers x 2 threads, low priority, 2.5 GB guard.
#   3. Watchdog: if free memory < 20%, stop job 2 first, then job 1.
set -uo pipefail
cd "$(dirname "$0")/.."
export PYTORCH_MPS_HIGH_WATERMARK_RATIO=0.35
log() { echo "$(date +%T) $*"; }

log "drafter start"
( scripts/guard.sh 28800 4000 -- .venv/bin/python scripts/train.py --set TDM --model transformer --epochs 3 \
    2>&1 | grep -vE "Warn|  step [0-9]+/" > logs/overnight_tdm_drafter.log; log "drafter done" ) &
DRAFTER=$!

log "expert targeting start"
( nice -n 10 scripts/guard.sh 30000 2500 -- .venv/bin/python scripts/train_ppo_play.py --name ppo4 --resume \
    --workers 2 --threads 2 --lam 0.95 --games 640 --lr 1e-5 --kl 0.01 --kl-final 0.01 --kl-iters 1 \
    > logs/overnight_ppo4.log 2>&1; log "expert targeting done" ) &
SIMS=$!

free_pct() { memory_pressure | awk -F': ' '/free percentage/{gsub("%","",$2); print $2}'; }
while kill -0 $DRAFTER 2>/dev/null || kill -0 $SIMS 2>/dev/null; do
  f=$(free_pct)
  if [ "${f:-100}" -lt 20 ]; then
    if kill -0 $SIMS 2>/dev/null; then
      log "memory ${f}% free: stopping expert targeting"; pkill -f "train_ppo_play.py --name ppo4"; sleep 120; continue
    elif kill -0 $DRAFTER 2>/dev/null; then
      log "memory ${f}% free: stopping the drafter"; pkill -f "train.py --set TDM"; sleep 120; continue
    fi
  fi
  sleep 60
done
log "overnight compute finished"
