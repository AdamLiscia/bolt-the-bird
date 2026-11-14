#!/usr/bin/env bash
# Waits for the PPO bot's round-robin (engine_pilot_newbot.sh rl_ppo345) to finish, then trains PPO
# from the human-imitation model (experiment 2: PPO vs REINFORCE from scratch), guarded 20 h.
set -u
cd "$(dirname "$0")/.."
while pgrep -f "engine_pilot_newbot.sh rl_ppo345" >/dev/null; do sleep 60; done
.venv/bin/python scripts/engine_pilot_validate.py newbot-analyze --name rl_ppo345 >/dev/null 2>&1
echo "$(date '+%F %T') evaluation over ($(tail -1 data/m7_rl_ppo345/newbot.log)); starting PPO from scratch" >>runs/chain_ppo_scratch.log
exec scripts/guard.sh 72000 6000 -- .venv/bin/python scripts/train_rl.py --reward played --algo ppo --device cpu \
  --iters 400 --played-eval-every 30 --name rl_ppo_scratch >runs/rl_ppo_scratch.log 2>&1
