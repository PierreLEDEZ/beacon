mod events;
pub mod install;
mod logging;
mod platform;
mod server;
mod session;

use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{error, info, warn};

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
    // Guard must outlive the runtime, so park it in the Tauri state.
    let log_guard = logging::init();
    info!("beacon starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            // Move the guard into Tauri's managed state so background
            // writer threads stay alive until the app exits.
            if let Some(g) = log_guard {
                app.manage(GuardHolder(g));
            }

            let handle = app.handle();

            if let Err(e) = platform::window::position_notch_top_center(handle) {
                error!(error = %e, "failed to position notch window");
            }
            if let Err(e) = platform::window::apply_noactivate(handle) {
                error!(error = %e, "failed to apply WS_EX_NOACTIVATE");
            }
            if let Err(e) = platform::hotkeys::register_toggle_shortcut(handle) {
                error!(error = %e, "failed to register global shortcut");
            }
            if let Err(e) = platform::tray::install(app) {
                error!(error = %e, "failed to install system tray");
            }

            let sessions = SessionManager::new();
            let events = EventBus::default();

            app.manage(sessions.clone());
            app.manage(events.clone());

            let emit_handle = handle.clone();
            let mut rx = events.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            if let Err(e) = emit_handle.emit(BUS_EVENT, &msg) {
                                error!(error = %e, "emit beacon://bus failed");
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!(dropped = n, "ipc subscriber lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            let sessions_for_server = sessions.clone();
            let events_for_server = events.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    server::serve(sessions_for_server, events_for_server, server::DEFAULT_PORT)
                        .await
                {
                    error!(error = %e, "http server exited with error");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![resize_notch, list_sessions])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Newtype so we can store the tracing_appender guard in Tauri's managed
/// state without clashing with any other transparent guard in the future.
struct GuardHolder(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);
