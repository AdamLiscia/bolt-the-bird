#!/usr/bin/env bash
# Plays the rollout pilot's remaining M7 games in guarded chunks until none are left or HOURS pass.
#
#     scripts/engine_pilot_overnight.sh [HOURS] [SET] [ROLLOUTS CANDIDATES]
#
# Defaults: 9 hours, set a (b, c: data/m7b, data/m7c), 4 rollouts x 6 candidates (games_rollout.txt);
# any other search size writes games_r<R>c<C>.txt.
#
# Every chunk goes through scripts/guard.sh (10 minutes, 2 GB): the runner stops starting games
# after 380 s and caps each game at 180 s, so a healthy chunk ends before the guard does. Any
# chunk the guard kills, or that fails, stops the whole run: nothing retries on its own. Progress
# goes to DIR/overnight.log; the analysis runs at the end.
set -u
cd "$(dirname "$0")/.."
hours=${1:-9}
set_=${2:-a}
ro=${3:-4}
ca=${4:-6}
name=rollout; [ "$ro $ca" != "4 6" ] && name=r${ro}c${ca}
d=data/m7; [ "$set_" != a ] && d=data/m7$set_
[ -n "${DIR:-}" ] && d=$DIR  # another bot's decks and jobs (engine_pilot_validate.py newbot)
aborts=0
end=$(($(date +%s) + hours * 3600))
log=$d/overnight.log
echo "$(date '+%F %T') start $name, until $(date -r "$end" '+%F %T')" >>"$log"
while [ "$(date +%s)" -lt "$end" ]; do
  out=$(scripts/guard.sh 600 2000 -- engine/target/release/examples/pilot_games \
    $d/decks.txt $d/jobs.txt $d/games_$name.txt rollout 380 8 "$ro" "$ca" 2>>"$log")
  rc=$?
  echo "$(date '+%F %T') rc=$rc $out" >>"$log"
  # An abort (134) is the rare memory corruption in QUESTIONS.md: the chunk's finished games are
  # saved, so retry, up to 3 times. Anything else, or a 4th abort, stops the run.
  if [ "$rc" -eq 134 ] && [ "$aborts" -lt 3 ]; then
    aborts=$((aborts + 1))
    echo "$(date '+%F %T') abort $aborts of 3 allowed; retrying" >>"$log"
    continue
  fi
  if [ "$rc" -ne 0 ]; then
    echo "$(date '+%F %T') stopped: chunk exited $rc" >>"$log"
    exit "$rc"
  fi
  case "$out" in *"; 0 of "*) break ;; esac
done
[ -z "${DIR:-}" ] && .venv/bin/python scripts/engine_pilot_validate.py analyze --set "$set_" >>"$log" 2>&1
echo "$(date '+%F %T') done" >>"$log"
