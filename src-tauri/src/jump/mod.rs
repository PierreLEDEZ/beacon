//! Jump-to-terminal orchestration.
//!
//! Two-step pipeline, each step best-effort:
//!   1. Focus the host terminal window via its cached HWND.
//!   2. Focus the right pane inside the multiplexer, if any, by calling
//!      `wsl.exe -d <distro> -- <mux cli>`.
//!
//! Missing data at either step is skipped silently — landing on the
//! terminal window is already a big win, and the user can navigate the
//! multiplexer manually.

use std::process::Command;

use crate::platform::hwnd::focus_hwnd;
use crate::session::{MultiplexerLocation, Session};

const DEFAULT_DISTRO: &str = "Ubuntu";

pub fn jump_to_session(session: &Session) -> JumpReport {
    let mut report = JumpReport::default();

    // Step 1: HWND focus.
    if let Some(h) = session.current_hwnd {
        match focus_hwnd(h) {
            Ok(()) => report.focused_window = true,
            Err(e) => {
                report.window_error = Some(e);
            }
        }
    } else {
        report.window_error = Some("no cached HWND for this session".into());
    }

    // Step 2: multiplexer pane focus (best-effort, only if we have mux info).
    if let Some(mux) = session.multiplexer.as_ref() {
        match jump_multiplexer(mux) {
            Ok(true) => report.focused_pane = true,
            Ok(false) => {} // unsupported multiplexer, silent
            Err(e) => {
                report.multiplexer_error = Some(e);
            }
        }
    }

    report
}

/// Returns Ok(true) if we issued a focus-pane command, Ok(false) if the
/// multiplexer kind isn't supported (silent skip), Err for an execution
/// failure (wsl.exe launched but failed to complete).
fn jump_multiplexer(mux: &MultiplexerLocation) -> Result<bool, String> {
    let distro = std::env::var("BEACON_WSL_DISTRO").unwrap_or_else(|_| DEFAULT_DISTRO.into());
    let cmd = match mux.kind.as_str() {
        "zellij" => {
            let session = mux.session.as_deref().unwrap_or("");
            let pane = mux.pane.as_deref().unwrap_or("");
            if session.is_empty() || pane.is_empty() {
                return Err("zellij mux location missing session/pane".into());
            }
            // `focus-pane-with-id` is the stable Zellij v0.40+ syntax.
            // `--session` targets the named session even when multiple
            // are running in the same WSL distro.
            format!(
                "zellij --session {} action focus-pane-with-id {}",
                shell_arg(session),
                shell_arg(pane)
            )
        }
        "tmux" => {
            // tmux pane id looks like "0" or "%2"; we send it as-is.
            let pane = mux.pane.as_deref().unwrap_or("");
            if pane.is_empty() {
                return Err("tmux mux location missing pane".into());
            }
            format!("tmux select-pane -t {}", shell_arg(pane))
        }
        _ => return Ok(false),
    };

    tracing::info!(mux = %mux.kind, cmd = %cmd, "jump: dispatching multiplexer focus");

    let status = Command::new("wsl.exe")
        .args(["-d", &distro, "--", "bash", "-lc", &cmd])
        .status()
        .map_err(|e| format!("wsl.exe spawn failed: {e}"))?;

    if !status.success() {
        return Err(format!("wsl.exe exited with {status}"));
    }
    Ok(true)
}

/// Minimal shell quoting for the short strings we pass to bash -lc.
/// Session / pane ids are well-formed in practice, but an adversarial
/// ZELLIJ_SESSION_NAME shouldn't let us inject arbitrary commands.
fn shell_arg(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// What the jump pipeline actually did, so the UI can surface partial
/// success / failures without treating "no mux" as an error.
#[derive(Debug, Default, serde::Serialize)]
pub struct JumpReport {
    pub focused_window: bool,
    pub focused_pane: bool,
    pub window_error: Option<String>,
    pub multiplexer_error: Option<String>,
}
