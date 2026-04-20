use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
};

use super::display;

pub const NOTCH_LABEL: &str = "notch";
const TOP_MARGIN_PX: i32 = 0;

pub const COLLAPSED_SIZE: (u32, u32) = (200, 32);
pub const EXPANDED_SIZE: (u32, u32) = (520, 320);

/// Place the notch at the top-center of the monitor the user is most
/// likely looking at — the one under their mouse cursor when the app
/// launches. Falls back to the primary monitor if we can't resolve that.
pub fn position_notch_top_center(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window(NOTCH_LABEL)
        .ok_or("notch window not found")?;

    let outer = window.outer_size()?;
    let pos = compute_initial_position(&window, outer)?;
    window.set_position(pos)?;
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

/// Resize the notch and keep it anchored top-center of the monitor it
/// currently lives on (do NOT jump to the cursor monitor on resize —
/// the user expects their notch to stay put).
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

    let outer = window.outer_size()?;
    let pos = compute_current_monitor_position(&window, outer)?;
    window.set_position(pos)?;
    Ok(())
}

/// At startup, prefer the monitor under the cursor (multi-screen setups
/// typically have the user's attention on whichever screen they were
/// just using). Fall back to the primary monitor, then to the window's
/// current monitor as a last resort.
fn compute_initial_position(
    window: &WebviewWindow,
    outer: PhysicalSize<u32>,
) -> Result<PhysicalPosition<i32>, Box<dyn std::error::Error>> {
    if let Some(area) = display::cursor_monitor_work_area() {
        let x = area.x + (area.width - outer.width as i32) / 2;
        let y = area.y + TOP_MARGIN_PX;
        return Ok(PhysicalPosition::new(x, y));
    }
    compute_current_monitor_position(window, outer)
}

/// For operations that must not relocate the window across monitors,
/// recenter relative to whichever monitor the window currently sits on.
fn compute_current_monitor_position(
    window: &WebviewWindow,
    outer: PhysicalSize<u32>,
) -> Result<PhysicalPosition<i32>, Box<dyn std::error::Error>> {
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .ok_or("no monitor available")?;
    let pos = monitor.position();
    let size = monitor.size();
    let x = pos.x + (size.width as i32 - outer.width as i32) / 2;
    let y = pos.y + (TOP_MARGIN_PX as f64 * monitor.scale_factor()) as i32;
    Ok(PhysicalPosition::new(x, y))
}
