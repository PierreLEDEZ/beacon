use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::server::dto::EventRequest;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Working,
    /// Blocking PreToolUse event awaits the user's Allow/Deny.
    WaitingApproval,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub claude_session_id: String,
    pub first_seen: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: Status,
    pub cwd: String,
    pub multiplexer: Option<MultiplexerLocation>,
    pub host_terminal: HostTerminal,
    pub last_event_type: Option<String>,
    pub last_tool_name: Option<String>,
    /// Raw HWND of the terminal window hosting this Claude session,
    /// captured on the first event. `None` on non-Windows or if the OS
    /// returned no foreground window at the capture moment.
    pub current_hwnd: Option<i64>,
    /// Basename (no `.exe`) of the host process owning `current_hwnd`.
    /// Fallback surface when the hook reports `host_terminal.kind ==
    /// "unknown"` (e.g. raw `wsl.exe` inside cmd/pwsh without WT_SESSION).
    pub terminal_exe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplexerLocation {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostTerminal {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markers: Option<serde_json::Value>,
}

#[derive(Clone, Default)]
pub struct SessionManager {
    inner: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a session from an incoming event. Returns the post-update session.
    pub fn upsert_from_event(&self, req: &EventRequest) -> Session {
        let now = Utc::now();
        let next_status = status_from_event(&req.event_type);
        // Capture BEFORE taking the write lock — GetForegroundWindow
        // should not depend on our own lock state.
        let captured_hwnd = crate::platform::hwnd::capture_foreground_hwnd();
        let captured_exe = captured_hwnd.and_then(crate::platform::hwnd::process_name_of_hwnd);
        let mut map = self.inner.write().expect("session lock poisoned");

        let entry = map
            .entry(req.claude.session_id.clone())
            .and_modify(|s| {
                s.last_activity = now;
                if let Some(new) = next_status {
                    s.status = new;
                }
                s.cwd = req.claude.cwd.clone();
                s.multiplexer = req.execution_context.multiplexer.clone();
                s.host_terminal = req.execution_context.host_terminal.clone();
                s.last_event_type = Some(req.event_type.clone());
                s.last_tool_name = req.claude.tool_name.clone();
                // Don't overwrite a previously-captured HWND: the first
                // event (SessionStart, usually) is the most reliable
                // moment and later events could fire while the user has
                // switched to another window.
                if s.current_hwnd.is_none() {
                    s.current_hwnd = captured_hwnd;
                    s.terminal_exe = captured_exe.clone();
                }
            })
            .or_insert_with(|| Session {
                claude_session_id: req.claude.session_id.clone(),
                first_seen: now,
                last_activity: now,
                status: next_status.unwrap_or(Status::Idle),
                cwd: req.claude.cwd.clone(),
                multiplexer: req.execution_context.multiplexer.clone(),
                host_terminal: req.execution_context.host_terminal.clone(),
                last_event_type: Some(req.event_type.clone()),
                last_tool_name: req.claude.tool_name.clone(),
                current_hwnd: captured_hwnd,
                terminal_exe: captured_exe,
            });
        entry.clone()
    }

    pub fn list(&self) -> Vec<Session> {
        let map = self.inner.read().expect("session lock poisoned");
        let mut sessions: Vec<Session> = map.values().cloned().collect();
        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        sessions
    }

    pub fn get(&self, claude_session_id: &str) -> Option<Session> {
        self.inner
            .read()
            .expect("session lock poisoned")
            .get(claude_session_id)
            .cloned()
    }

    /// Force a session's status; used when a blocking event lands
    /// (WaitingApproval) or resolves (back to Working).
    pub fn set_status(&self, claude_session_id: &str, status: Status) -> Option<Session> {
        let mut map = self.inner.write().expect("session lock poisoned");
        let s = map.get_mut(claude_session_id)?;
        s.status = status;
        s.last_activity = Utc::now();
        Some(s.clone())
    }
}

/// Map a Claude Code hook event name to the session status it implies.
/// Returns None for events that should update `last_activity` without changing status.
fn status_from_event(event_type: &str) -> Option<Status> {
    match event_type {
        "SessionStart" => Some(Status::Idle),
        // PreToolUse is blocking in Phase 2: it's set explicitly to
        // WaitingApproval by the route handler, not here.
        "UserPromptSubmit" | "PostToolUse" => Some(Status::Working),
        "Stop" | "SubagentStop" | "SessionEnd" => Some(Status::Done),
        _ => None,
    }
}
