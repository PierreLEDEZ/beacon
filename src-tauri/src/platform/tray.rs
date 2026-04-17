use tauri::{
    menu::{Menu, MenuEvent, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager,
};

use super::window::NOTCH_LABEL;

/// Install the Beacon system-tray icon with a Show / Hide / Quit menu.
/// Left-clicking the tray icon toggles the notch's visibility — the menu
/// stays useful for users who prefer explicit actions (and works with
/// keyboard-only navigation).
pub fn install(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit Beacon", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &hide_i, &quit_i])?;

    let icon = app
        .default_window_icon()
        .ok_or("default window icon missing")?
        .clone();

    TrayIconBuilder::with_id("beacon-tray")
        .tooltip("Beacon")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_icon_event)
        .build(app)?;

    Ok(())
}

fn handle_menu_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
        "show" => show_notch(app),
        "hide" => hide_notch(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn handle_icon_event<R: tauri::Runtime>(tray: &tauri::tray::TrayIcon<R>, event: TrayIconEvent) {
    // Only fire on the release half of a left click; ignore hovers/right
    // clicks (right-click is the native menu shortcut on Windows).
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_notch(tray.app_handle());
    }
}

fn show_notch<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window(NOTCH_LABEL) {
        let _ = window.show();
    }
}

fn hide_notch<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window(NOTCH_LABEL) {
        let _ = window.hide();
    }
}

fn toggle_notch<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(window) = app.get_webview_window(NOTCH_LABEL) else {
        return;
    };
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        Ok(false) | Err(_) => {
            let _ = window.show();
        }
    }
}
