#!/usr/bin/env bash
# Plays the rollout pilot's remaining M7 games in guarded chunks until none are left or HOURS pass.
#
#     scripts/engine_pilot_overnight.sh [HOURS]      # default 9
#
# Every chunk goes through scripts/guard.sh (10 minutes, 2 GB): the runner stops starting games
# after 380 s and caps each game at 180 s, so a healthy chunk ends before the guard does. Any
# chunk the guard kills, or that fails, stops the whole run: nothing retries on its own. Progress
# goes to data/m7/overnight.log; the analysis runs at the end.
set -u
cd "$(dirname "$0")/.."
hours=${1:-9}
end=$(($(date +%s) + hours * 3600))
log=data/m7/overnight.log
echo "$(date '+%F %T') start, until $(date -r "$end" '+%F %T')" >>"$log"
while [ "$(date +%s)" -lt "$end" ]; do
  out=$(scripts/guard.sh 600 2000 -- engine/target/release/examples/pilot_games \
    data/m7/decks.txt data/m7/jobs.txt data/m7/games_rollout.txt rollout 380 8 2>>"$log")
  rc=$?
  echo "$(date '+%F %T') rc=$rc $out" >>"$log"
  if [ "$rc" -ne 0 ]; then
    echo "$(date '+%F %T') stopped: chunk exited $rc" >>"$log"
    exit "$rc"
  fi
  case "$out" in *"; 0 of "*) break ;; esac
done
.venv/bin/python scripts/engine_pilot_validate.py analyze >>"$log" 2>&1
echo "$(date '+%F %T') done" >>"$log"
