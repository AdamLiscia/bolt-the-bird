#!/usr/bin/env bash
# Waits for the running evaluation chain, then plays the deck tournament (engine_tournament.py):
# rules-based first (minutes), then smart 4 x 6 (hours), guarded and resumable, then the analysis.
set -u
cd "$(dirname "$0")/.."
log=runs/tournament.log
while pgrep -f "chain_eval_best.sh|engine_pilot_newbot.sh" >/dev/null; do sleep 60; done
echo "$(date '+%F %T') evaluation over; tournament starts" >>"$log"
d=data/tournament
scripts/guard.sh 900 2000 -- engine/target/release/examples/pilot_games $d/decks.txt $d/jobs.txt \
  $d/games_baseline.txt baseline 800 8 >>"$log" 2>&1
DIR=$d scripts/engine_pilot_overnight.sh 14 t 4 6
.venv/bin/python scripts/engine_tournament.py analyze >>"$log" 2>&1
echo "$(date '+%F %T') tournament done" >>"$log"
