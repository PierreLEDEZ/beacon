mod events;
mod platform;
mod server;
mod session;

use tauri::{command, AppHandle, Manager};

use crate::events::EventBus;
use crate::session::SessionManager;

#[command]
async fn resize_notch(app: AppHandle, expanded: bool) -> Result<(), String> {
    platform::window::resize_notch(&app, expanded).map_err(|e| e.to_string())
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

            // Spawn the axum server as a tokio task. A port-in-use error is
            // logged but does not abort app startup — the UI still launches.
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
        .invoke_handler(tauri::generate_handler![resize_notch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
