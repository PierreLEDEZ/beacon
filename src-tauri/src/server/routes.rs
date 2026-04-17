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
    let session = state.sessions.upsert_from_event(&req);
    state
        .events
        .publish(BusMessage::SessionUpdated { session });
    Ok(Json(EventResponse {
        event_id,
        accepted: true,
    }))
}
