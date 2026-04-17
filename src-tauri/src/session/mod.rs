use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::server::dto::EventRequest;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Working,
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
            });
        entry.clone()
    }

    pub fn list(&self) -> Vec<Session> {
        let map = self.inner.read().expect("session lock poisoned");
        let mut sessions: Vec<Session> = map.values().cloned().collect();
        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        sessions
    }
}

/// Map a Claude Code hook event name to the session status it implies.
/// Returns None for events that should update `last_activity` without changing status.
fn status_from_event(event_type: &str) -> Option<Status> {
    match event_type {
        "SessionStart" => Some(Status::Idle),
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => Some(Status::Working),
        "Stop" | "SubagentStop" | "SessionEnd" => Some(Status::Done),
        _ => None,
    }
}
