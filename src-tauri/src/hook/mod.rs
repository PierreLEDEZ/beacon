//! `beacon.exe hook` — the Windows-native counterpart of the bash
//! `beacon-hook`. Used by Claude Code Desktop on Windows, which can't
//! invoke the WSL bash script directly.
//!
//! Responsibilities mirror the bash version:
//!   - read Claude Code's hook JSON from stdin
//!   - POST an enriched payload to the local Beacon server
//!   - for PreToolUse: long-poll `/wait/<id>` and translate the
//!     server's decision into Claude Code's stdout schema
//!   - never break Claude Code: exit 0 on any error

use std::io::Read;
use std::time::Duration;

use serde_json::{json, Value};

const DEFAULT_URL: &str = "http://127.0.0.1:37421";
const POST_TIMEOUT_SECS: u64 = 2;
const WAIT_TIMEOUT_SECS: u64 = 330;

pub fn run() {
    // Release builds of beacon.exe use the `windows` subsystem (no
    // console). Attach to the parent so any println in the blocking
    // branch actually reaches Claude Code's stdout pipe.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
        return;
    }

    let raw: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = raw
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if event_type.is_empty() {
        return;
    }
    let blocking = event_type == "PreToolUse";

    let url = std::env::var("BEACON_URL").unwrap_or_else(|_| DEFAULT_URL.into());

    let payload = json!({
        "event_type": event_type,
        "blocking": blocking,
        "claude": {
            "session_id": raw.get("session_id").cloned().unwrap_or(Value::Null),
            "cwd": raw.get("cwd").cloned().unwrap_or(Value::Null),
            "transcript_path": raw.get("transcript_path").cloned().unwrap_or(Value::Null),
            "tool_name": raw.get("tool_name").cloned().unwrap_or(Value::Null),
            "tool_input": raw.get("tool_input").cloned().unwrap_or(Value::Null),
        },
        "execution_context": {
            // Claude Desktop is a native Windows GUI, no multiplexer.
            "multiplexer": Value::Null,
            "host_terminal": {
                "kind": "claude-desktop",
                "markers": {}
            }
        }
    });

    if !blocking {
        let _ = ureq::post(&format!("{url}/event"))
            .timeout(Duration::from_secs(POST_TIMEOUT_SECS))
            .send_json(payload);
        return;
    }

    // Blocking branch: capture event_id, long-poll for decision.
    let resp = match ureq::post(&format!("{url}/event"))
        .timeout(Duration::from_secs(POST_TIMEOUT_SECS))
        .send_json(payload)
    {
        Ok(r) => r,
        Err(_) => return,
    };

    let body: Value = match resp.into_json() {
        Ok(v) => v,
        Err(_) => return,
    };
    let event_id = match body.get("event_id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return,
    };

    let decision: Value = match ureq::get(&format!("{url}/wait/{event_id}"))
        .timeout(Duration::from_secs(WAIT_TIMEOUT_SECS))
        .call()
        .and_then(|r| r.into_json().map_err(Into::into))
    {
        Ok(v) => v,
        Err(_) => return,
    };

    let kind = decision.get("decision").and_then(|v| v.as_str()).unwrap_or("");
    let reason = decision
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let translated = match kind {
        "allow" => "approve",
        "deny" => "block",
        _ => return,
    };

    // Emit Claude Code's stdout schema verbatim.
    let out = json!({ "decision": translated, "reason": reason });
    println!("{}", out);
}
