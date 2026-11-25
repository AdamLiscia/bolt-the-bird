# The desktop simulation farm: jobs that survive a power cut

Status: **design, 2026-11-25. Nothing is built or running.** The desktop's own Claude session builds the
machine side once Adam has set it up and said go.

The desktop (i9-9900K, 16 threads, 32 GB, AMD GPU so CPU only, WSL2) lost power five times between
September and November, for hours to days each time. Adam's requirement: **every job on it survives a power
cut** without losing more than a few minutes of work and without counting anything twice.

## The pieces

| Piece | Where | Reuses |
|---|---|---|
| `bolt.jobs`: a job wrapper (checkpoint, resume, batches) | `src/bolt/jobs.py` (new) | `bolt.backup` (S3 upload), `train.py`'s per-epoch checkpoints and `--resume` |
| A job file per run: what to run and its state on S3 | `jobs/<run>.json` in the repo, state on S3 | |
| The runner: starts every job not marked done | `scripts/farm.py` (new) | `scripts/guard.sh` (time and memory caps) |
| The restart hook | a systemd user service in WSL | |
| Credentials | AWS profile `bolt-desktop`: Put, Get and List on the bucket, **no delete** | the bucket from `bolt.backup` |

## What a checkpoint holds

One file per checkpoint, `torch.save` of a dict:

- `model`, `optimizer`, `scheduler`: state dicts (whatever the job has; a pure simulation job has none);
- `rng`: Python's `random`, NumPy's and torch's generator states, so a resumed run continues the same
  random stream rather than repeating or skipping one;
- `step`: the job's own counter (epoch, PPO iteration, or batch index);
- `done_batches`: the IDs of the simulation batches whose results are already saved (see below);
- `job`, `git`, `time`: the job's name, the commit it ran, and when.

## Writing a checkpoint so a cut never corrupts it

Order matters, and nothing is ever deleted (the key can't):

1. Write locally to `ckpt/<run>/tmp.pt`, `fsync`, then rename to `ckpt/<run>/step_<n>.pt`. A rename is
   atomic, so a power cut leaves either the old set of files or the new one, never half a file.
2. Upload that file to `s3://<bucket>/farm/<run>/step_<n>_<utc>.pt`: a new, unique key each time.
3. Read it back from S3 (a HEAD request: size and checksum) to confirm the whole file landed.
4. Only then overwrite `s3://<bucket>/farm/<run>/latest.json` with
   `{"key": ..., "step": n, "sha256": ..., "time": ...}`, and write the same pointer locally with the same
   write-then-rename.

`latest.json` only ever points at a checkpoint that's complete on S3. A cut during steps 1–3 leaves the
old pointer and the old checkpoint, so a few minutes are lost and nothing breaks. Overwriting
`latest.json` is a PutObject, which the no-delete policy allows, and bucket versioning keeps every old
pointer anyway.

**When:** every 10–15 minutes (a `--checkpoint-every` setting, checked between steps, never mid-step),
on clean exit, and on SIGTERM (WSL shutting down cleanly). A step that runs longer than the interval
checkpoints after it finishes.

## Resuming

On start, a job:

1. reads the local `latest.json`, then S3's, and takes whichever has the higher `step`, so a machine that
   lost its disk can still resume from S3;
2. downloads that checkpoint if it isn't local, checks its `sha256`, and loads it; if the check fails it
   falls back to the previous checkpoint (S3's object versions of `latest.json` list them);
3. restores the model, optimizer, scheduler and RNG states, and the `done_batches` set;
4. continues from `step + 1`.

With no checkpoint it starts fresh. `train.py --resume` becomes a thin call into this.

## Simulation batches: idempotent, never double-counted

Simulation jobs (self-play, the drafting experiment, the Foundations targeting sims) work in **batches** with
stable IDs: `<run>/b<index>`, where the index fixes the batch's seeds (e.g. `seed = base_seed + index`).

- A batch writes its results to its own key, `farm/<run>/results/b<index>.json`, written locally then
  uploaded, whole.
- A batch's ID goes into `done_batches` only after its results upload is confirmed.
- On resume, any batch not in `done_batches` runs again **with the same seeds**, so it produces the same
  results and overwrites the same key. A batch cut halfway is re-run whole, never counted twice.
- Totals (win rates and so on) are always recomputed from the result files, never kept as running sums, so
  a re-run can't double a count.

## The runner and the restart hook

`scripts/farm.py` reads `jobs/*.json` and starts, one at a time per its CPU budget (default 12 of 16 threads,
leaving Windows usable), every job whose S3 `latest.json` doesn't say `"done": true`. Each job runs under
`guard.sh` with its time and memory caps. A finished job writes `"done": true` into its pointer.

In WSL, a systemd user service runs the runner at boot and restarts it if it exits with an error:

```ini
# ~/.config/systemd/user/bolt-farm.service
[Unit]
Description=Bolt simulation farm
After=network-online.target

[Service]
WorkingDirectory=%h/bolt-the-bird
ExecStart=%h/bolt-the-bird/.venv/bin/python scripts/farm.py
Restart=on-failure
RestartSec=60
Environment=AWS_PROFILE=bolt-desktop

[Install]
WantedBy=default.target
```

WSL starts only when something asks for it, so a Windows Task Scheduler task "at log on" (or "at startup",
with WSL set to run without a login) runs `wsl -d Ubuntu --exec /bin/true`. That boots WSL, and systemd
starts the service. `systemctl --user enable --now bolt-farm` plus `loginctl enable-linger` make it run
without a terminal open.

## Things to decide with Adam before it's built

- The CPU budget (12 of 16 threads is the default here) and whether jobs may run while he's using the PC.
- Whether the desktop should pause for his streams the way the Mac does (likely not: it isn't the streaming
  machine).
- The first jobs: Tarkir self-play once the play network is padded for Tarkir; the drafting experiment;
  the Foundations targeting sims.
