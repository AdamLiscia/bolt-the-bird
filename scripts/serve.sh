#!/usr/bin/env bash
# Runs the Bolt service on 127.0.0.1:8766 (docs/api.md). The shared secret lives in ~/.config/bolt/key
# (mode 600, outside the repo); Nerd Leagues sends it as X-Bolt-Key. 12-hour limit, 1.5 GB cap.
set -euo pipefail
cd "$(dirname "$0")/.."
KEY_FILE="${HOME}/.config/bolt/key"
if [ ! -s "$KEY_FILE" ]; then
  mkdir -p "$(dirname "$KEY_FILE")"
  ( umask 077; .venv/bin/python -c "import secrets; print(secrets.token_urlsafe(32))" > "$KEY_FILE" )
  echo "created $KEY_FILE"
fi
BOLT_KEY="$(cat "$KEY_FILE")" exec scripts/guard.sh 43200 1500 -- .venv/bin/python -m bolt.service
