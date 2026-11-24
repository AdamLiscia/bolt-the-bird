#!/usr/bin/env bash
# Tarkir: Dragonstorm human-style drafter (2026-11-24). Baselines, then the MLP (quick check), then the
# transformer, 3 epochs, as for Foundations. One job at a time, memory-capped, logs to logs/.
set -uo pipefail
cd "$(dirname "$0")/.."
echo "== baselines $(date +%T)"
scripts/guard.sh 1800 4000 -- .venv/bin/python scripts/baselines.py --set TDM 2>&1 | grep -v Warn | tail -8
echo "== mlp $(date +%T)"
scripts/guard.sh 7200 5000 -- .venv/bin/python scripts/train.py --set TDM --model mlp --epochs 2 2>&1 | grep -vE "Warn|step " | tail -6
echo "== transformer $(date +%T)"
scripts/guard.sh 28800 6000 -- .venv/bin/python scripts/train.py --set TDM --model transformer --epochs 3 2>&1 | grep -vE "Warn" | grep -vE "  step [0-9]+/" | tail -10
echo "== done $(date +%T)"
