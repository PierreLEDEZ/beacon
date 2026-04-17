mod events;
pub mod install;
mod platform;
mod server;
mod session;

use tauri::{command, AppHandle, Emitter, Manager, State};

use crate::events::EventBus;
use crate::session::{Session, SessionManager};

pub const BUS_EVENT: &str = "beacon://bus";

#[command]
async fn resize_notch(app: AppHandle, expanded: bool) -> Result<(), String> {
    platform::window::resize_notch(&app, expanded).map_err(|e| e.to_string())
}

#[command]
fn list_sessions(sessions: State<'_, SessionManager>) -> Vec<Session> {
    sessions.list()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();

            if let Err(e) = platform::window::position_notch_top_center(handle) {
                eprintln!("[beacon] failed to position notch window: {e}");
            }
            if let Err(e) = platform::window::apply_noactivate(handle) {
                eprintln!("[beacon] failed to apply WS_EX_NOACTIVATE: {e}");
            }

            let sessions = SessionManager::new();
            let events = EventBus::default();

            app.manage(sessions.clone());
            app.manage(events.clone());

            // Forward every bus message to the webview so the React state
            // stays in sync without polling. Uses a dedicated subscriber so
            // the axum server and the UI are independent consumers.
            let emit_handle = handle.clone();
            let mut rx = events.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            if let Err(e) = emit_handle.emit(BUS_EVENT, &msg) {
                                eprintln!("[beacon] emit {BUS_EVENT} failed: {e}");
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            eprintln!("[beacon] ipc subscriber lagged, dropped {n} messages");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            // Spawn the axum server as its own tokio task. A port-in-use
            // error is logged but does not abort app startup.
            let sessions_for_server = sessions.clone();
            let events_for_server = events.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    server::serve(sessions_for_server, events_for_server, server::DEFAULT_PORT)
                        .await
                {
                    eprintln!("[beacon] http server exited with error: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![resize_notch, list_sessions])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
