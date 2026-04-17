# Beacon

> Windows-side pseudo-notch dashboard for Claude Code sessions running in WSL.

Beacon puts a small always-on-top pill at the top of your screen that shows every active Claude Code session you have open across your WSL terminals (Ghostty / Zellij, tmux, VS Code integrated terminal, Windows Terminal, WezTerm, …). Phase 1 is the **read-only socle**: it surfaces session activity in real time. Phase 2 adds blocking permission / question prompts; Phase 3 adds jump-to-terminal, sounds, and history.

See `docs/ARCHITECTURE.md` for the full design.

## Prerequisites

- Windows 11 with WSL2 (mirrored networking recommended) and an Ubuntu distro
- Inside WSL: `curl`, `jq` (`sudo apt install jq`), `bash`
- On Windows: Node ≥ 20, Rust ≥ 1.77, MSVC Build Tools, WebView2 runtime

## Build & run

```bash
# From a fresh PowerShell
cd E:\Dev\Beacon
npm install
npm run tauri dev     # dev mode with HMR
# or
npm run tauri build   # release .msi in src-tauri/target/release/bundle/
```

## Install the hooks (one-time)

1. **From Windows**, push the shell scripts into WSL:

   ```powershell
   # In dev
   .\src-tauri\target\debug\beacon.exe --install-hooks
   # Or release
   .\src-tauri\target\release\beacon.exe --install-hooks
   ```

   This drops `beacon-hook` and `beacon-install-hooks` into `~/.local/bin/` inside your WSL Ubuntu (override distro via `BEACON_WSL_DISTRO=Other`).

2. **From WSL**, run the settings merger (requires `jq`):

   ```bash
   ~/.local/bin/beacon-install-hooks
   ```

   This merges the 5 Phase-1 hooks into `~/.claude/settings.json` without clobbering your existing `enabledPlugins`, `permissions`, or other hook entries. Idempotent — safe to re-run.

3. Launch `claude` inside any WSL terminal. A card should appear in the Beacon notch within ~1 second.

## Keyboard shortcuts

| Shortcut                 | Action                                     |
| ------------------------ | ------------------------------------------ |
| `Ctrl+Alt+Shift+Space`   | Show/hide the notch                        |
| `Ctrl+Alt+Shift+Y`       | Allow the oldest pending prompt            |
| `Ctrl+Alt+Shift+N`       | Deny the oldest pending prompt             |
| *(tray left-click)*      | Toggle visibility                          |

## Protocol (HTTP, `127.0.0.1:37421`)

| Method | Route                  | Purpose                                              |
| ------ | ---------------------- | ---------------------------------------------------- |
| GET    | `/health`              | Liveness ping (`{"status":"ok"}`)                    |
| POST   | `/event`               | Accept a Claude Code hook event                      |
| GET    | `/sessions`            | List active sessions                                 |
| GET    | `/pending`             | List blocking prompts currently awaiting a decision  |
| GET    | `/wait/:event_id`      | Long-poll by the hook; returns decision or timeout   |
| POST   | `/decision/:event_id`  | Frontend resolves a prompt (allow / deny / answer)   |

Phase 3 will add `/jump/:id`, history endpoints, etc.

## Environment knobs

| Variable            | Default                              | Consumed by                   |
| ------------------- | ------------------------------------ | ----------------------------- |
| `BEACON_LOG`        | `info`                               | Rust backend (tracing filter) |
| `BEACON_WSL_DISTRO` | `Ubuntu`                             | `--install-hooks`             |
| `BEACON_URL`        | `http://127.0.0.1:37421`             | hook script                   |
| `BEACON_TIMEOUT`    | `2`                                  | hook script (curl seconds, POST)|
| `BEACON_WAIT_TIMEOUT`| `330`                               | hook script (long-poll seconds) |
| `BEACON_HOOK_DEBUG` | unset                                | hook script                   |
| `BEACON_HOOK_LOG`   | `$HOME/.cache/beacon/hook.log`       | hook script                   |

## Logs

- Rust side: `%APPDATA%\Beacon\logs\beacon.log` (daily rotation, INFO by default).
- Hook side: only when `BEACON_HOOK_DEBUG=1` — path defaults to `~/.cache/beacon/hook.log`.

## Troubleshooting

- **No session appears after running `claude`.** Turn on the hook's debug log:
  ```bash
  BEACON_HOOK_DEBUG=1 tail -f ~/.cache/beacon/hook.log
  ```
  Re-run `claude`; you should see a `POST http://127.0.0.1:37421/event …` line per hook invocation. If `curl` fails, verify Beacon is running and try `curl http://127.0.0.1:37421/health` from WSL.
- **Port 37421 already in use.** Beacon logs the bind failure but the UI still launches; nothing will flow from WSL. Identify the conflicting process with `Get-NetTCPConnection -LocalPort 37421` (PowerShell) and either free the port or relaunch Beacon. (Phase 3 will make the port configurable.)
- **Notch doesn't appear.** Check `%APPDATA%\Beacon\logs\beacon.log` for `failed to position notch window` — usually a monitor-detection edge case. Hitting `Ctrl+Alt+Shift+Space` toggles visibility in case it's just hidden.
- **`--install-hooks` says "access denied" writing to WSL.** Make sure the distro is running (`wsl --list -v` → `Running`) and your user can write to `$HOME/.local/bin`.
- **Settings file turned into a single line after install.** Normal: `jq` pretty-prints JSON; Claude Code accepts either shape.

## Phase 1 end-to-end checklist

1. `npm run tauri build` produces an exe that launches to a 200×32 notch centered at the top of the screen.
2. `Ctrl+Alt+Shift+Space` hides/shows it; tray menu (right-click) works; left-click the tray toggles.
3. From WSL: `curl http://127.0.0.1:37421/health` → `{"status":"ok"}`.
4. `beacon.exe --install-hooks` and then `~/.local/bin/beacon-install-hooks` in WSL → settings merged, existing keys intact.
5. `claude` in a Zellij pane in Ghostty → session card appears with `ghostty · zellij · …`.
6. `claude` in VS Code integrated terminal → card with `vscode-term`.
7. Typing a prompt flips the status dot to working (amber); `Stop` flips it to done (grey).
8. Log file populated at `%APPDATA%\Beacon\logs\beacon.log`.

## License

TBD.
