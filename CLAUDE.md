# CLAUDE.md — Bolt the Bird

An AI that drafts Magic: The Gathering, a Rust rules engine that plays it, a learned player, and a small
web service (`/v1`) that Nerd Leagues uses. Status and decisions: `PLAN.md`. Every public number:
`docs/headline.json` (+ `docs/WRITEUPS.md`, `scripts/check_writeups.py`).

## Working here

- Python: `.venv/bin/python`, tests `.venv/bin/python -m pytest -q`; lint `ruff check .` and
  `ruff format --check .` (CI runs both, plus `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`).
- Rust needs `export PATH=/opt/homebrew/opt/rustup/bin:$PATH`. `make engine-py` rebuilds and installs the
  Python engine. **Never install an engine build while a batch job (training, evaluation) is running on the
  installed one.**
- Every long run goes through `scripts/guard.sh <seconds> <MB> -- ...` (time and memory caps, kills the
  whole process tree). **One heavy job at a time** (the Mac froze with two, 2026-11-18). Keep every
  checkpoint.
- **Checkpoints back up to S3** (Adam's bucket `nerdleagues-bolt-checkpoints-011834`, us-east-1, AWS
  profile `bolt-backup`; approved 2026-11-25). `src/bolt/backup.py`: training scripts call
  `upload_quietly(path)` after each save (never raises; `BOLT_BACKUP=0` skips), and
  `python -m bolt.backup sync [--until HH:MM]` uploads anything missing. Results go to
  `checkpoints/UPLOADED.jsonl`. **The credentials can't delete, by design**: never ask for, add or use
  delete rights, and never copy the key anywhere. Uploads share his bandwidth, so pause them during his
  streams (about 14:45 on stream days).
- Say "smart player", never "search player" (search means searching your library in Magic).

## The Bolt service (Nerd Leagues talks to it)

- `scripts/serve.sh` runs it on `127.0.0.1:8766` under the guard (12 h, 1.5 GB). The key is in
  `~/.config/bolt/key` (mode 600): never print, paste or commit it.
- **`docs/api.md` is a contract with Nerd Leagues.** Changes are additive only, and tell the NL
  orchestrator session ("Bolt the Bird integration with Nerd Leagues") when something ships.
  Its ledger: `~/nerd-leagues-master/nerd-leagues/docs/bolt-status.md`.
- **Restart rule:** restart 8766 only when `GET /v1/health` shows `games_live: 0` in the same check, or
  when Adam has said games may end; tell the orchestrator after. Restarting ends every game.
- The service never touches Nerd Leagues' database.
