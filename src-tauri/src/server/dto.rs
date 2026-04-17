use serde::{Deserialize, Serialize};

use crate::session::{HostTerminal, MultiplexerLocation};

/// Payload posted by the beacon-hook bash script for every Claude Code event.
/// Fields mirror docs/ARCHITECTURE.md section 6.
#[derive(Debug, Deserialize)]
pub struct EventRequest {
    pub event_type: String,
    #[serde(default)]
    pub blocking: bool,
    pub claude: ClaudePayload,
    pub execution_context: ExecutionContext,
}

#[derive(Debug, Deserialize)]
pub struct ClaudePayload {
    pub session_id: String,
    #[serde(default)]
    pub pid: Option<u32>,
    pub cwd: String,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionContext {
    #[serde(default)]
    pub shell_pid: Option<u32>,
    #[serde(default)]
    pub tty: Option<String>,
    #[serde(default)]
    pub multiplexer: Option<MultiplexerLocation>,
    pub host_terminal: HostTerminal,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub event_id: String,
    pub accepted: bool,
}
