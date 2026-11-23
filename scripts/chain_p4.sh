#!/usr/bin/env bash
# P4 (2026-11-23 night): re-grade the drafting bots with the trained player (play_v2, iteration 850)
# on both sides. Waits until START (default 21:00), after the day's admin testing. One job at a time,
# 4 workers, 3 GB cap and a time limit per step; every step is resumable (rerun to continue).
#   1. the tournament: 5 bots' decks from the same 512 tables, every pairing (102,400 games)
#   2. field a round-robin: the judge-trained and human-model decks vs 64-deck field a (65,408)
#   3. the PPO bot's (round 345) decks vs the same field (32,704)
set -uo pipefail
cd "$(dirname "$0")/.."
START=${START:-21:00}
while [ "$(date +%H:%M)" \< "$START" ]; do sleep 60; done
run() {
  echo "== $1 $(date +%T)"
  scripts/guard.sh "$2" 3000 -- .venv/bin/python scripts/play_jobs_network.py "$3/decks.txt" "$3/jobs.txt" \
    "$3/games_network.txt" --workers 4 --hours "$4" 2>&1 | grep -v Warn
  echo "== $1 stopped (exit ${PIPESTATUS[0]}) $(date +%T)"
}
run tournament 14400 data/tournament 3.8
run field_a 10800 data/m7 2.8
run ppo345 7200 data/m7_rl_ppo345/a 1.8
echo "== analyze $(date +%T)"
.venv/bin/python scripts/engine_tournament.py analyze
.venv/bin/python scripts/engine_pilot_validate.py analyze --set a
.venv/bin/python scripts/engine_pilot_validate.py newbot-analyze --name rl_ppo345
echo "== done $(date +%T)"
