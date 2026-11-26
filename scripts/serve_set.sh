#!/usr/bin/env bash
# Runs a one-set Bolt service under the guard (12 h, 1.5 GB), for launchd (docs/launchd.md) or by hand:
#   scripts/serve_set.sh <SET> <PORT> <worktree> <venv>
#   scripts/serve_set.sh CMD 8768 ~/bolt-cmd-serve ~/bolt-cmd-venv
# The key is read from ~/.config/bolt/key at start, never written anywhere.
set -euo pipefail
SET="$1"; PORT="$2"; TREE="$3"; VENV="$4"
cd "$TREE"
mkdir -p logs
export BOLT_KEY="$(cat "$HOME/.config/bolt/key")" BOLT_SET="$SET" BOLT_PORT="$PORT" PYTHONPATH="$TREE/src"
exec bash scripts/guard.sh 43200 1500 -- "$VENV/bin/python" -m bolt.service
