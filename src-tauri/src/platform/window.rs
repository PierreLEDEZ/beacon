use tauri::{AppHandle, Manager, PhysicalPosition};

pub const NOTCH_LABEL: &str = "notch";
const TOP_MARGIN_PX: i32 = 0;

/// Position the notch window at the top-center of the monitor it was
/// initially placed on (or the primary monitor as fallback) and show it.
pub fn position_notch_top_center(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window(NOTCH_LABEL)
        .ok_or("notch window not found")?;

    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .ok_or("no monitor available")?;

    // Both monitor.size() and window.outer_size() are in physical pixels.
    let screen_w = monitor.size().width as i32;
    let outer = window.outer_size()?;
    let x_physical = (screen_w - outer.width as i32) / 2;
    let y_physical = (TOP_MARGIN_PX as f64 * monitor.scale_factor()) as i32;

    window.set_position(PhysicalPosition::new(x_physical, y_physical))?;
    window.show()?;
    Ok(())
}
