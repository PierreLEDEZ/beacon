//! `beacon.exe --install-hooks` implementation.
//!
//! Two targets:
//!
//! 1. **WSL Claude Code** (CLI) — drop the bash hook scripts into
//!    `~/.local/bin/` inside the configured WSL distro so the existing
//!    `beacon-hook` settings.json merger can pick them up.
//!
//! 2. **Claude Code Desktop** (Windows) — register `beacon.exe hook` as
//!    the hook command in `%USERPROFILE%\.claude\settings.json`. Runs
//!    natively, no WSL involvement; the binary itself is the hook
//!    runner (see `src/hook/mod.rs`).
//!
//! Both targets are best-effort: if WSL is unreachable (distro
//! missing) or the Windows settings.json fails to merge, we log the
//! specific error and continue with the other target.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const HOOK_SCRIPT: &str = include_str!("../../../hooks/beacon-hook");
const INSTALLER_SCRIPT: &str = include_str!("../../../hooks/beacon-install-hooks");

const DEFAULT_DISTRO: &str = "Ubuntu";

/// Phase-2 Claude Code hook events registered on both targets.
const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SessionEnd",
];

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let distro = std::env::var("BEACON_WSL_DISTRO").unwrap_or_else(|_| DEFAULT_DISTRO.to_string());

    println!("Beacon install-hooks — two targets follow.");
    println!();

    // --- WSL ---------------------------------------------------------
    println!("[1/2] WSL ({distro}): pushing shell scripts into ~/.local/bin/.");
    match install_wsl(&distro) {
        Ok(()) => {
            println!();
            println!("  done. From inside WSL, run this to merge the hooks into");
            println!("  ~/.claude/settings.json:");
            println!();
            println!("      ~/.local/bin/beacon-install-hooks");
            println!();
            println!("  (requires jq: `sudo apt install jq`)");
        }
        Err(e) => {
            eprintln!("  WSL step failed: {e}");
            eprintln!("  (skipping — Windows install will still run)");
        }
    }
    println!();

    // --- Windows / Claude Desktop -----------------------------------
    println!("[2/2] Windows: registering `beacon.exe hook` in %USERPROFILE%\\.claude\\settings.json.");
    match install_windows() {
        Ok(path) => {
            println!("  merged hooks into {}", path.display());
            println!("  Claude Code Desktop will pick them up on its next launch.");
        }
        Err(e) => {
            eprintln!("  Windows step failed: {e}");
        }
    }

    Ok(())
}

// ---------- WSL branch ------------------------------------------------

fn install_wsl(distro: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_to_wsl(distro, "$HOME/.local/bin/beacon-hook", HOOK_SCRIPT)?;
    write_to_wsl(
        distro,
        "$HOME/.local/bin/beacon-install-hooks",
        INSTALLER_SCRIPT,
    )?;
    Ok(())
}

/// Pipe `contents` via stdin to `cat > <dest>` inside WSL. Using cat+
/// stdin avoids any path-translation headaches and `cp` ownership quirks.
fn write_to_wsl(
    distro: &str,
    dest: &str,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let normalized = contents.replace("\r\n", "\n");

    let cmd = format!(
        r#"mkdir -p "$(dirname {dest})" && cat > {dest} && chmod +x {dest}"#
    );

    let mut child = Command::new("wsl.exe")
        .args(["-d", distro, "--", "bash", "-lc", &cmd])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn wsl.exe: {e}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("wsl.exe stdin unavailable")?;
        stdin.write_all(normalized.as_bytes())?;
    }
    drop(child.stdin.take());

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("wsl.exe exited with {status}").into());
    }

    println!("  wrote {dest}");
    Ok(())
}

// ---------- Windows / Claude Desktop branch --------------------------

fn install_windows() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let settings_path = dirs::home_dir()
        .ok_or("could not resolve %USERPROFILE%")?
        .join(".claude")
        .join("settings.json");

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut settings = load_json_or_default(&settings_path)?;

    let cmd = format!("\"{}\" hook", exe.display());
    merge_hooks_for_all_events(&mut settings, &cmd);

    let serialized = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&settings_path, serialized)?;
    Ok(settings_path)
}

fn load_json_or_default(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    Ok(serde_json::from_str(&raw)?)
}

/// For each Phase-2 event, drop any existing matcher block that already
/// references our bin (idempotent re-install) and append our entry.
/// Every other key in the settings object is preserved verbatim.
fn merge_hooks_for_all_events(settings: &mut Value, command: &str) {
    let obj = settings
        .as_object_mut()
        .expect("settings root is not a JSON object");
    let hooks = obj
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("hooks is not a JSON object");

    for event in HOOK_EVENTS {
        let existing = hooks
            .entry(event.to_string())
            .or_insert_with(|| json!([]));
        let arr = existing
            .as_array_mut()
            .expect("hooks[event] is not an array");

        arr.retain(|entry| {
            let Some(inner) = entry.get("hooks").and_then(|h| h.as_array()) else {
                return true;
            };
            !inner
                .iter()
                .any(|h| h.get("command").and_then(|c| c.as_str()) == Some(command))
        });

        arr.push(json!({
            "hooks": [{
                "type": "command",
                "command": command,
            }]
        }));
    }
}
