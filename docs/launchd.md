# The services under launchd (auto-restart)

Adam chose this on 2026-11-25. **Not installed until he confirms it directly.** The three services restart on their
own about 30 s after any exit, including after the 12 h guard ends a run, and start again after a reboot. The
guard still caps each run at 12 hours and 1.5 GB; launchd only adds the restart.

| Label | Port | Set | Runs | Log |
|---|---|---|---|---|
| `com.bolt.service-8766` | 8766 | FDN | `~/bolt-the-bird/scripts/serve.sh` | `~/bolt-the-bird/logs/service.log` |
| `com.bolt.service-8767` | 8767 | TDM | `scripts/serve_set.sh TDM 8767 ~/bolt-tdm-serve ~/bolt-tdm-venv` | `~/bolt-tdm-serve/logs/serve_8767.log` |
| `com.bolt.service-8768` | 8768 | CMD | `scripts/serve_set.sh CMD 8768 ~/bolt-cmd-serve ~/bolt-cmd-venv` | `~/bolt-cmd-serve/logs/serve_8768.log` |

The plists are in `ops/launchd/`. Each sets `KeepAlive`, `RunAtLoad` and `ThrottleInterval` 30. The secret key is
read from `~/.config/bolt/key` when a service starts; it isn't in any plist.

## Install (one port at a time, with `games_live` 0)

```sh
# stop the copy started by hand first (its guard, then the server), then:
cp ops/launchd/com.bolt.service-8768.plist ~/Library/LaunchAgents/
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.bolt.service-8768.plist
curl -s -H "X-Bolt-Key: $(cat ~/.config/bolt/key)" http://127.0.0.1:8768/v1/health
```

## Deploying a new build

Update the worktree and venv as before, then restart the service (launchd brings it straight back):

```sh
launchctl kickstart -k gui/$UID/com.bolt.service-8768
```

## Remove

```sh
launchctl bootout gui/$UID/com.bolt.service-8768
rm ~/Library/LaunchAgents/com.bolt.service-8768.plist
```

`launchctl print gui/$UID/com.bolt.service-8768` shows whether it's loaded and its last exit status.
