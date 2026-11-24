#!/usr/bin/env bash
# After a crash or reboot: pick tonight's runs up where they stopped. Nothing is lost but the batch in
# flight: P4's tournaments skip games already written, training resumes from its last saved round.
#   scripts/resume_tonight.sh        (starts in the background, logs to logs/resume_tonight.log)
set -uo pipefail
cd "$(dirname "$0")/.."
if pgrep -f "scripts/chain_p4b.sh|scripts/chain_ppo4.sh|train_ppo_play|play_jobs_network" >/dev/null; then
  echo "already running: nothing to resume"; exit 0
fi
nohup bash -c 'scripts/chain_p4b.sh && scripts/chain_ppo4.sh' > logs/resume_tonight.log 2>&1 &
echo "resumed (pid $!): tail -f logs/resume_tonight.log"
