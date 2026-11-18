#!/usr/bin/env bash
# Waits for the P1 training chain, then plays the P2 evaluation with the three best turn models.
set -u
cd "$(dirname "$0")/.."
while pgrep -f "chain_turns.sh" >/dev/null; do sleep 60; done
for n in cast1 attack1 block1; do
  [ -f "checkpoints/turns/$n/best.pt" ] || { echo "$(date '+%F %T') missing $n; P2 not run" >>runs/chain_p2.log; exit 1; }
done
echo "$(date '+%F %T') P2 evaluation starts" >>runs/chain_p2.log
scripts/guard.sh 10800 6000 -- .venv/bin/python scripts/eval_imitation.py --pairs 1500 --workers 6 >>runs/chain_p2.log 2>&1
echo "$(date '+%F %T') P2 evaluation exited $?" >>runs/chain_p2.log
