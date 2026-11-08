#!/usr/bin/env bash
# The strength ladder: weaker search pilots through the full round-robin on fields a, b and c, one
# run after another, until DEADLINE (epoch seconds). Each run is scripts/engine_pilot_overnight.sh
# (guarded chunks). The next run starts only if the last one finished every game.
set -u
cd "$(dirname "$0")/.."
deadline=$1
log=data/m7/ladder.log
for cfg in "1 2" "2 4"; do
  set -- $cfg
  for s in a b c; do
    d=data/m7; [ "$s" != a ] && d=data/m7$s
    hours=$(( (deadline - $(date +%s)) / 3600 ))
    if [ "$hours" -lt 1 ]; then echo "$(date '+%F %T') out of time before r$1c$2 $s" >>"$log"; exit 0; fi
    echo "$(date '+%F %T') r$1c$2 field $s, up to $hours h" >>"$log"
    scripts/engine_pilot_overnight.sh "$hours" "$s" "$1" "$2"
    jobs=$(wc -l <"$d/jobs.txt")
    got=$(awk 'NF==3' "$d/games_r$1c$2.txt" 2>/dev/null | wc -l)
    echo "$(date '+%F %T') r$1c$2 field $s: $got of $jobs games" >>"$log"
    [ "$got" -eq "$jobs" ] || { echo "$(date '+%F %T') stopped: unfinished" >>"$log"; exit 1; }
    .venv/bin/python scripts/engine_pilot_validate.py analyze --set "$s" >/dev/null 2>>"$log"
  done
done
echo "$(date '+%F %T') ladder done" >>"$log"
