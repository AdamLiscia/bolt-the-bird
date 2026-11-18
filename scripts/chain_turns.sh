#!/usr/bin/env bash
# Trains the three P1 turn models one after another, each guarded (4 h, 6 GB) and resumable:
# rerunning this script after a crash resumes whichever model was in progress.
set -u
cd "$(dirname "$0")/.."
for task in cast attack block; do
  name=${task}1
  if [ -f "results/turns_$name.json" ] && grep -q '"test"' "results/turns_$name.json"; then continue; fi
  resume=""; [ -d "checkpoints/turns/$name" ] && resume="--resume"
  echo "$(date '+%F %T') $task starts $resume" >>runs/chain_turns.log
  scripts/guard.sh 14400 6000 -- .venv/bin/python scripts/train_turns.py --task "$task" --name "$name" --epochs 2 $resume \
    >>"runs/turns_$name.log" 2>&1
  echo "$(date '+%F %T') $task exited $?" >>runs/chain_turns.log
done
