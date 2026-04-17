mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Err(e) = platform::window::position_notch_top_center(app.handle()) {
                eprintln!("[beacon] failed to position notch window: {e}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
