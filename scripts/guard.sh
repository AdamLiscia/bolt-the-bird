#!/usr/bin/env bash
# Runs a command with a wall-clock limit and a memory cap over its whole process tree.
#
#     scripts/guard.sh <seconds> <max-MB> -- <command...>
#
# With GUARD_ACTIVE_TIME=1 the time limit counts only seconds in which the command isn't stopped
# (SIGSTOP, e.g. the desktop farm pausing while Adam games), so a long pause doesn't use it up.
#
# Kills everything the command started, and exits 124 (time) or 125 (memory), when either limit
# is passed. On 2026-11-05 an engine loop that never ended grew two fuzz runs to ~42 GB each and
# took down a 16 GB laptop; long engine runs go through this.
set -u
if [ $# -lt 4 ] || [ "$3" != "--" ]; then
  echo "usage: $0 <seconds> <max-MB> -- <command...>" >&2
  exit 2
fi
limit_s=$1
limit_kb=$(($2 * 1024))
shift 3

"$@" &
root=$!

# The command and every process under it.
tree() {
  ps -A -o pid=,ppid= | awk -v root="$root" '
    { parent[$1] = $2 }
    END {
      keep[root] = 1
      changed = 1
      while (changed) {
        changed = 0
        for (p in parent) if (!(p in keep) && (parent[p] in keep)) { keep[p] = 1; changed = 1 }
      }
      for (p in keep) print p
    }'
}

kill_tree() {
  pids=$(tree)
  # shellcheck disable=SC2086
  kill -TERM $pids 2>/dev/null
  sleep 1
  # shellcheck disable=SC2086
  kill -KILL $pids 2>/dev/null
}

# Stopping the guard (kill, Ctrl-C) stops everything it started: otherwise the command lives on
# without its limits (2026-11-23: a stopped guard left the Bolt service holding its port).
trap 'kill_tree; exit 143' TERM INT HUP

start=$(date +%s)
active=0
while kill -0 "$root" 2>/dev/null; do
  pids=$(tree | paste -sd, -)
  rss=$(ps -o rss= -p "$pids" 2>/dev/null | awk '{s += $1} END {print s + 0}')
  if [ "$rss" -gt "$limit_kb" ]; then
    echo "guard: memory over $(($limit_kb / 1024)) MB ($(($rss / 1024)) MB); killed" >&2
    kill_tree
    exit 125
  fi
  if [ "${GUARD_ACTIVE_TIME:-0}" = 1 ]; then
    case "$(ps -o stat= -p "$root" 2>/dev/null)" in
      T*) ;;  # stopped: this second doesn't count
      *) active=$((active + 1)) ;;
    esac
    used=$active
  else
    used=$(($(date +%s) - start))
  fi
  if [ "$used" -ge "$limit_s" ]; then
    echo "guard: over $limit_s s; killed" >&2
    kill_tree
    exit 124
  fi
  sleep 1
done
wait "$root"
