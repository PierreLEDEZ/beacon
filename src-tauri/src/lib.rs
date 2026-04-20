mod decisions;
mod events;
pub mod install;
mod jump;
mod logging;
mod platform;
mod server;
mod session;
mod settings;

use tauri::{command, AppHandle, Emitter, Manager, State};
use tracing::{error, info, warn};

use crate::decisions::{PendingDecisions, PendingEvent};
use crate::events::EventBus;
use crate::session::{Session, SessionManager};
use crate::settings::{Settings, SettingsStore};

pub const BUS_EVENT: &str = "beacon://bus";

#[command]
async fn resize_notch(app: AppHandle, expanded: bool) -> Result<(), String> {
    platform::window::resize_notch(&app, expanded).map_err(|e| e.to_string())
}

#[command]
fn list_sessions(sessions: State<'_, SessionManager>) -> Vec<Session> {
    sessions.list()
}

#[command]
fn list_pending(pending: State<'_, PendingDecisions>) -> Vec<PendingEvent> {
    pending.list()
}

#[command]
fn jump_session(
    sessions: State<'_, SessionManager>,
    settings: State<'_, SettingsStore>,
    claude_session_id: String,
) -> Result<jump::JumpReport, String> {
    let session = sessions
        .get(&claude_session_id)
        .ok_or_else(|| format!("unknown session: {claude_session_id}"))?;
    Ok(jump::jump_to_session(&session, &settings.get()))
}

#[command]
fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

#[command]
fn update_settings(
    store: State<'_, SettingsStore>,
    settings: Settings,
) -> Result<Settings, String> {
    store.update(settings)
}

/// Frontend-driven path for resolving a pending prompt. Uses Tauri IPC
/// rather than the HTTP route so the webview doesn't have to worry
/// about CORS on its own origin. The HTTP /decision/:id route stays
/// available for out-of-process callers (curl, tests, future clients).
#[command]
fn resolve_pending(
    pending: State<'_, PendingDecisions>,
    events: State<'_, EventBus>,
    event_id: String,
    decision: decisions::DecisionInput,
) -> Result<(), String> {
    let decision: decisions::Decision = decision.into();
    if !pending.resolve(&event_id, decision.clone()) {
        return Err(format!("unknown or already resolved: {event_id}"));
    }
    events.publish(events::BusMessage::PendingResolved {
        event_id,
        decision,
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Guard must outlive the runtime, so park it in the Tauri state.
    let log_guard = logging::init();
    info!("beacon starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
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
            if let Err(e) = platform::hotkeys::register_all(handle) {
                error!(error = %e, "failed to register global shortcuts");
            }
            if let Err(e) = platform::tray::install(app) {
                error!(error = %e, "failed to install system tray");
            }

            let sessions = SessionManager::new();
            let events = EventBus::default();
            let pending = PendingDecisions::new();
            let settings_store = SettingsStore::load_or_default();

            app.manage(sessions.clone());
            app.manage(events.clone());
            app.manage(pending.clone());
            app.manage(settings_store.clone());

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
            let pending_for_server = pending.clone();
            let port = settings_store.get().port;
            let timeout_secs = settings_store.get().decision_timeout_secs;
            tauri::async_runtime::spawn(async move {
                if let Err(e) = server::serve(
                    sessions_for_server,
                    events_for_server,
                    pending_for_server,
                    port,
                    timeout_secs,
                )
                .await
                {
                    error!(error = %e, "http server exited with error");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            resize_notch,
            list_sessions,
            list_pending,
            resolve_pending,
            jump_session,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Newtype so we can store the tracing_appender guard in Tauri's managed
/// state without clashing with any other transparent guard in the future.
struct GuardHolder(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);
