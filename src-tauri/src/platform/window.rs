use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition};

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
};

pub const NOTCH_LABEL: &str = "notch";
const TOP_MARGIN_PX: i32 = 0;

pub const COLLAPSED_SIZE: (u32, u32) = (200, 32);
pub const EXPANDED_SIZE: (u32, u32) = (520, 320);

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

/// Apply `WS_EX_NOACTIVATE` so clicking the notch never steals focus from the
/// active foreground window (terminal, editor, etc.). Applied after the window
/// is shown so the extended style sticks.
#[cfg(windows)]
pub fn apply_noactivate(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window(NOTCH_LABEL)
        .ok_or("notch window not found")?;
    let hwnd = window.hwnd()?;

    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new = current | WS_EX_NOACTIVATE.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new);
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_noactivate(_app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

/// Resize the notch and re-anchor it to the top-center so the pill appears
/// to grow from its centerline rather than sliding sideways.
pub fn resize_notch(app: &AppHandle, expanded: bool) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window(NOTCH_LABEL)
        .ok_or("notch window not found")?;

    let (w, h) = if expanded {
        EXPANDED_SIZE
    } else {
        COLLAPSED_SIZE
    };
    window.set_size(LogicalSize::new(w, h))?;

    // Re-center x (keep y at top).
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .ok_or("no monitor available")?;
    let outer = window.outer_size()?;
    let screen_w = monitor.size().width as i32;
    let x_physical = (screen_w - outer.width as i32) / 2;
    let y_physical = (TOP_MARGIN_PX as f64 * monitor.scale_factor()) as i32;
    window.set_position(PhysicalPosition::new(x_physical, y_physical))?;
    Ok(())
}
