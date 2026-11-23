#!/usr/bin/env bash
# P4 fields b and c (2026-11-23): the same round-robins as chain_p4.sh's field a, on the two
# replication fields, so the re-graded gaps pool over all three fields like the original results.
# One job at a time, 4 workers, 3 GB cap; resumable.
set -uo pipefail
cd "$(dirname "$0")/.."
run() {
  echo "== $1 $(date +%T)"
  scripts/guard.sh "$2" 3000 -- .venv/bin/python scripts/play_jobs_network.py "$3/decks.txt" "$3/jobs.txt" \
    "$3/games_network.txt" --workers 4 --hours "$4" 2>&1 | grep -v Warn
  echo "== $1 stopped (exit ${PIPESTATUS[0]}) $(date +%T)"
}
run field_b 10800 data/m7b 2.8
run field_c 10800 data/m7c 2.8
run ppo345_b 7200 data/m7_rl_ppo345/b 1.8
run ppo345_c 7200 data/m7_rl_ppo345/c 1.8
echo "== analyze $(date +%T)"
.venv/bin/python scripts/engine_pilot_validate.py analyze --set b
.venv/bin/python scripts/engine_pilot_validate.py analyze --set c
.venv/bin/python scripts/engine_pilot_validate.py newbot-analyze --name rl_ppo345
echo "== done $(date +%T)"
