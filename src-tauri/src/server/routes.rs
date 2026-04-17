use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use uuid::Uuid;

use crate::events::{BusMessage, EventBus};
use crate::server::dto::{EventRequest, EventResponse};
use crate::session::{Session, SessionManager};

#[derive(Clone)]
pub struct AppState {
    pub sessions: SessionManager,
    pub events: EventBus,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/event", post(ingest_event))
        .route("/sessions", get(list_sessions))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<Session>> {
    Json(state.sessions.list())
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
        cwd = %req.claude.cwd,
        "event received"
    );

    let session = state.sessions.upsert_from_event(&req);
    state
        .events
        .publish(BusMessage::SessionUpdated { session });
    Ok(Json(EventResponse {
        event_id,
        accepted: true,
    }))
}
