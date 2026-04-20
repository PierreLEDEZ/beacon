pub mod dto;
pub mod routes;

use std::net::SocketAddr;

use axum::http::{header, HeaderValue, Method};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::decisions::PendingDecisions;
use crate::events::EventBus;
use crate::session::SessionManager;

pub use routes::AppState;

pub const DEFAULT_PORT: u16 = 37421;

/// Bind the HTTP server to 127.0.0.1:<port> and serve until the task is cancelled.
/// Any bind failure is propagated so the caller can log + surface it (the UI
/// should still launch even if the port is taken, per docs §13).
pub async fn serve(
    sessions: SessionManager,
    events: EventBus,
    pending: PendingDecisions,
    port: u16,
    decision_timeout_secs: u64,
) -> Result<(), std::io::Error> {
    let state = AppState {
        sessions,
        events,
        pending,
        decision_timeout_secs,
    };
    let app = routes::router(state).layer(cors_layer());

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "http server listening");
    axum::serve(listener, app).await
}

/// Restrict browser-origin requests to localhost variants. Hooks sent from
/// the WSL bash script do not set an Origin header and are therefore not
/// subject to CORS checks — this layer only guards against a stray browser
/// tab trying to POST to our local server.
fn cors_layer() -> CorsLayer {
    let origins = [
        "http://127.0.0.1".parse::<HeaderValue>().unwrap(),
        "http://localhost".parse::<HeaderValue>().unwrap(),
        "tauri://localhost".parse::<HeaderValue>().unwrap(),
    ];
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
}
