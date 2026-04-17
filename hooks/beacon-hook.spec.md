# beacon-hook contract

Generic Claude Code hook that forwards events to the local Beacon HTTP server.
Designed to be installed once and stay generic: no multiplexer or terminal
name is hardcoded in the hook path registered in `~/.claude/settings.json`.

## I/O

- **stdin**: Claude Code's hook JSON (schema: `docs.claude.com/claude-code/hooks`).
- **stdout**: empty in Phase 1. Phase 2 (blocking hooks) will emit a JSON
  decision object.
- **stderr**: silent unless `BEACON_HOOK_DEBUG=1`.
- **exit code**: always 0, no matter what. Claude Code must never be broken
  by a misconfigured Beacon — this is enforced via `trap 'exit 0' EXIT`.

## Environment

| Variable            | Default                              | Purpose                 |
| ------------------- | ------------------------------------ | ----------------------- |
| `BEACON_URL`        | `http://127.0.0.1:37421`             | Beacon server base URL  |
| `BEACON_TIMEOUT`    | `2`                                  | curl `--max-time` (sec) |
| `BEACON_HOOK_DEBUG` | unset                                | set `1` to enable logs  |
| `BEACON_HOOK_LOG`   | `$HOME/.cache/beacon/hook.log`       | log file path           |

## Detection heuristics

### Multiplexer (in order)

1. `$ZELLIJ` non-empty → `kind=zellij`, pulls `$ZELLIJ_SESSION_NAME`, `$ZELLIJ_PANE_ID`.
2. `$TMUX` non-empty → `kind=tmux`, pulls `#S / #I / #P` via `tmux display-message`.
3. `$STY` non-empty → `kind=screen`, pulls `$STY`.
4. Otherwise → `null`.

### Host terminal (in order)

1. `$TERM_PROGRAM=vscode` → `vscode-term`.
2. `$GHOSTTY_RESOURCES_DIR` or `$TERM=xterm-ghostty` → `ghostty`.
3. `$WEZTERM_EXECUTABLE` → `wezterm`.
4. `$WT_SESSION` → `windows-terminal`.
5. Otherwise → `unknown`.

The raw env markers that triggered the match are echoed under
`execution_context.host_terminal.markers` for debugging.

## Outbound payload

```json
{
  "event_type": "<hook_event_name>",
  "claude": {
    "session_id": "...",
    "cwd": "...",
    "transcript_path": "...",
    "tool_name": "..." | null,
    "tool_input": {...} | null
  },
  "execution_context": {
    "multiplexer": {"kind":"zellij|tmux|screen","session":"...","tab":"...","pane":"..."} | null,
    "host_terminal": {"kind":"...","markers":{...}}
  }
}
```

Matches the shape Beacon expects in `server/dto.rs`.

## Dependencies

- `bash` (shebang: `/usr/bin/env bash`)
- `curl` (ubiquitous; required)
- `jq` (strongly recommended; fallback is best-effort regex and drops `tool_input`)
- `tmux` CLI (only when running inside tmux, to resolve session/window/pane)
