#!/usr/bin/env bash
# Plays a new bot's decks (engine_pilot_validate.py newbot) through the round-robins: smart 1 x 2
# on fields a, b, c, then smart 4 x 6 on a, b, c, until DEADLINE (epoch seconds). Each run is
# engine_pilot_overnight.sh (guarded chunks, up to 3 abort retries); the next starts only if the
# last finished every game.
#     scripts/engine_pilot_newbot.sh NAME DEADLINE
set -u
cd "$(dirname "$0")/.."
name=$1
deadline=$2
log=data/m7_$name/newbot.log
for cfg in "1 2" "4 6"; do
  set -- $cfg
  out=r$1c$2; [ "$cfg" = "4 6" ] && out=rollout
  for s in a b c; do
    d=data/m7_$name/$s
    hours=$(( (deadline - $(date +%s)) / 3600 ))
    if [ "$hours" -lt 1 ]; then echo "$(date '+%F %T') out of time before $out $s" >>"$log"; exit 0; fi
    echo "$(date '+%F %T') $out field $s, up to $hours h" >>"$log"
    DIR=$d scripts/engine_pilot_overnight.sh "$hours" "$s" "$1" "$2"
    jobs=$(wc -l <"$d/jobs.txt")
    got=$(awk 'NF==3' "$d/games_$out.txt" 2>/dev/null | wc -l)
    echo "$(date '+%F %T') $out field $s: $got of $jobs games" >>"$log"
    [ "$got" -eq "$jobs" ] || { echo "$(date '+%F %T') stopped: unfinished" >>"$log"; exit 1; }
    .venv/bin/python scripts/engine_pilot_validate.py newbot-analyze --name "$name" >/dev/null 2>>"$log"
  done
done
echo "$(date '+%F %T') newbot done" >>"$log"
