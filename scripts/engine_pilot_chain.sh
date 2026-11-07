#!/usr/bin/env bash
# Waits for the running set-b driver, then runs set c until the shared deadline, only if b finished
# cleanly (its log ends in "done"). Deadline: the epoch seconds in $1.
set -u
cd "$(dirname "$0")/.."
deadline=$1
while pgrep -f "engine_pilot_overnight.sh 10 b" >/dev/null; do sleep 60; done
tail -1 data/m7b/overnight.log | grep -q " done$" || exit 1
hours=$(( (deadline - $(date +%s)) / 3600 ))
[ "$hours" -ge 1 ] && exec scripts/engine_pilot_overnight.sh "$hours" c
