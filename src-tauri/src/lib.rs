mod platform;

use tauri::{command, AppHandle};

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![resize_notch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
