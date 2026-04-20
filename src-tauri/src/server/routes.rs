use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::decisions::{Decision, DecisionInput, PendingDecisions, PendingEvent};
use crate::events::{BusMessage, EventBus};
use crate::history::{EventRecord, History};
use crate::jump::{jump_to_session, JumpReport};
use crate::settings::Settings;
use crate::server::dto::{EventRequest, EventResponse};
use crate::session::{Session, SessionManager, Status};

#[derive(Clone)]
pub struct AppState {
    pub sessions: SessionManager,
    pub events: EventBus,
    pub pending: PendingDecisions,
    pub history: Option<History>,
    pub decision_timeout_secs: u64,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/event", post(ingest_event))
        .route("/sessions", get(list_sessions))
        .route("/sessions/{claude_session_id}/history", get(list_session_history))
        .route("/pending", get(list_pending))
        .route("/wait/{event_id}", get(wait_decision))
        .route("/decision/{event_id}", post(post_decision))
        .route("/jump/{claude_session_id}", post(post_jump))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<Session>> {
    Json(state.sessions.list())
}

async fn list_pending(State(state): State<AppState>) -> Json<Vec<PendingEvent>> {
    Json(state.pending.list())
}

async fn ingest_event(
    State(state): State<AppState>,
    Json(req): Json<EventRequest>,
) -> Result<Json<EventResponse>, (StatusCode, String)> {
    let event_id = Uuid::new_v4().to_string();

    let host = req.execution_context.host_terminal.kind.as_str();
    let mux = req
        .execution_context
        .multiplexer
        .as_ref()
        .map(|m| m.kind.as_str())
        .unwrap_or("none");
    tracing::info!(
        event_id = %event_id,
        event_type = %req.event_type,
        session_id = %req.claude.session_id,
        host_terminal = host,
        multiplexer = mux,
        tool = req.claude.tool_name.as_deref().unwrap_or(""),
        blocking = req.blocking,
        cwd = %req.claude.cwd,
        "event received"
    );

    let session = state.sessions.upsert_from_event(&req);

    // Mirror the event into the history DB. Best-effort; a bad write
    // logs at warn level but never disrupts the live flow.
    if let Some(history) = state.history.as_ref() {
        let record = EventRecord {
            id: 0,
            event_id: event_id.clone(),
            session_id: req.claude.session_id.clone(),
            event_type: req.event_type.clone(),
            tool_name: req.claude.tool_name.clone(),
            cwd: req.claude.cwd.clone(),
            created_at: chrono::Utc::now(),
            metadata: req
                .claude
                .tool_input
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
        };
        history.record(&record);
    }

    // Blocking branch: flip the session to WaitingApproval, stash a
    // pending entry, and broadcast it so the UI can render the prompt.
    if req.blocking {
        state
            .sessions
            .set_status(&req.claude.session_id, Status::WaitingApproval);

        let pending = PendingEvent {
            event_id: event_id.clone(),
            session_id: req.claude.session_id.clone(),
            event_type: req.event_type.clone(),
            cwd: req.claude.cwd.clone(),
            tool_name: req.claude.tool_name.clone(),
            tool_input: req.claude.tool_input.clone(),
            created_at: Utc::now(),
        };
        state.pending.register(pending.clone());
        state
            .events
            .publish(BusMessage::PendingAwaiting { pending });
    }

    state
        .events
        .publish(BusMessage::SessionUpdated { session });

    Ok(Json(EventResponse {
        event_id,
        accepted: true,
    }))
}

/// Long-poll endpoint called by the hook. Blocks until a decision is
/// posted, or DEFAULT_TIMEOUT_SECS elapse (auto-deny).
async fn wait_decision(
    Path(event_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let Some(rx) = state.pending.take_receiver(&event_id) else {
        return (
            StatusCode::NOT_FOUND,
            format!("unknown or already consumed event_id: {event_id}"),
        )
            .into_response();
    };

    match tokio::time::timeout(Duration::from_secs(state.decision_timeout_secs), rx).await {
        Ok(Ok(decision)) => Json(decision).into_response(),
        Ok(Err(_)) => {
            // Sender side was dropped (e.g. server shutdown); treat as deny.
            Json(Decision::timeout_deny()).into_response()
        }
        Err(_elapsed) => {
            tracing::warn!(event_id = %event_id, "decision timed out, auto-deny");
            let decision = Decision::timeout_deny();
            state.pending.drop_meta(&event_id);
            // Notify the UI so it can drop the prompt card.
            state.events.publish(BusMessage::PendingResolved {
                event_id: event_id.clone(),
                decision: decision.clone(),
            });
            Json(decision).into_response()
        }
    }
}

/// Endpoint the frontend hits when the user clicks Allow/Deny.
async fn post_decision(
    Path(event_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<DecisionInput>,
) -> Response {
    let decision: Decision = body.into();
    let ok = state.pending.resolve(&event_id, decision.clone());
    if !ok {
        return (
            StatusCode::NOT_FOUND,
            format!("unknown or already resolved: {event_id}"),
        )
            .into_response();
    }

    tracing::info!(
        event_id = %event_id,
        decision = ?decision.decision,
        "decision resolved"
    );

    state.events.publish(BusMessage::PendingResolved {
        event_id: event_id.clone(),
        decision,
    });

    // Ask the session's state to fall back to Working — PreToolUse has
    // landed, user approved, Claude will now run the tool.
    // Note: we don't have the session_id from the path, so we look it up
    // via… actually we already cleared the pending meta. For now the next
    // event (PostToolUse or Tool result) will flip status correctly. The
    // UI also clears it immediately via the bus message.

    StatusCode::NO_CONTENT.into_response()
}

/// Trigger the jump pipeline for a session (HWND focus + multiplexer
/// pane). Always returns the JumpReport so the caller can see partial
/// success: missing-HWND or unsupported-multiplexer aren't errors.
async fn post_jump(
    Path(claude_session_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<JumpReport>, (StatusCode, String)> {
    let session = state.sessions.get(&claude_session_id).ok_or((
        StatusCode::NOT_FOUND,
        format!("unknown session: {claude_session_id}"),
    ))?;
    tracing::info!(
        claude_session_id = %claude_session_id,
        hwnd = ?session.current_hwnd,
        multiplexer = ?session.multiplexer.as_ref().map(|m| m.kind.as_str()),
        "jump request"
    );
    // The HTTP jump route doesn't currently propagate live settings — it
    // uses the same defaults as fresh installs. Frontend callers go
    // through the Tauri command which reads SettingsStore; external
    // curl jumps are a debug convenience, so defaults are acceptable.
    Ok(Json(jump_to_session(&session, &Settings::default())))
}

/// Read the last N events for a session from the SQLite history. If
/// the DB failed to open at startup, returns an empty array (not 500)
/// — history is a convenience, its absence shouldn't look like an error.
async fn list_session_history(
    Path(claude_session_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<EventRecord>>, (StatusCode, String)> {
    let Some(history) = state.history.as_ref() else {
        return Ok(Json(Vec::new()));
    };
    history
        .list_for_session(&claude_session_id, 200)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

type Response = axum::response::Response;
