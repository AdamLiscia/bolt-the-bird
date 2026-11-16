#!/usr/bin/env bash
# After a training run ends: pick its checkpoint with the best played eval and put it through the
# three round-robins (engine_pilot_validate.py newbot, then engine_pilot_newbot.sh).
#     scripts/chain_eval_best.sh RUN_NAME EVAL_NAME DEADLINE_EPOCH
set -u
cd "$(dirname "$0")/.."
run=$1 name=$2 deadline=$3
log=runs/chain_eval_$name.log
while pgrep -f "train_rl.py.*--name $run" >/dev/null; do sleep 60; done
best=$(grep -E "PLAYED EVAL it [0-9]+:" runs/$run.log | sed -E 's/.*it ([0-9]+): gain ([+-][0-9.]+).*/\2 \1/' \
  | while read -r g it; do [ -f "checkpoints/bot_${run}_it$it.pt" ] && echo "$g $it"; done | sort -g | tail -1 | awk '{print $2}')
if [ -z "$best" ]; then echo "$(date '+%F %T') no kept checkpoint with a played eval" >>"$log"; exit 1; fi
echo "$(date '+%F %T') best checkpoint: iteration $best ($(grep -E "PLAYED EVAL it $best:" runs/$run.log))" >>"$log"
scripts/guard.sh 900 6000 -- .venv/bin/python scripts/engine_pilot_validate.py newbot \
  --checkpoint "bot_${run}_it$best.pt" --name "$name" >>"$log" 2>&1 || exit 1
for s in a b c; do
  d=data/m7_$name/$s
  scripts/guard.sh 600 2000 -- engine/target/release/examples/pilot_games "$d/decks.txt" "$d/jobs.txt" \
    "$d/games_baseline.txt" baseline 500 8 >>"$log" 2>&1
done
exec scripts/engine_pilot_newbot.sh "$name" "$deadline"
