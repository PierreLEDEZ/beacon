use tauri::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, Wry,
};
use tauri_plugin_autostart::ManagerExt;

use super::window::NOTCH_LABEL;

/// Kept in Tauri-managed state so the menu event handler can tick /
/// untick the checkbox when autostart toggles.
struct AutostartMenuRef(CheckMenuItem<Wry>);

/// Install the Beacon system-tray icon. Left-click toggles the notch,
/// the menu gives Show / Hide / Start-with-Windows / Quit.
pub fn install(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;

    let autostart_checked = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(false);
    let autostart_i = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start with Windows",
        true,
        autostart_checked,
        None::<&str>,
    )?;

    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit Beacon", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_i, &hide_i, &autostart_i, &sep, &quit_i],
    )?;

    // Hold onto the check item so we can update it from handle_menu_event.
    app.manage(AutostartMenuRef(autostart_i.clone()));

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
        "autostart" => toggle_autostart(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn handle_icon_event<R: tauri::Runtime>(tray: &tauri::tray::TrayIcon<R>, event: TrayIconEvent) {
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

/// Switch the Windows login-launch registry entry on/off and keep the
/// tray checkbox in sync.
fn toggle_autostart<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let launcher = app.autolaunch();
    let was_enabled = launcher.is_enabled().unwrap_or(false);
    let result = if was_enabled {
        launcher.disable()
    } else {
        launcher.enable()
    };
    match result {
        Ok(()) => {
            let now_enabled = !was_enabled;
            if let Some(menu_ref) = app.try_state::<AutostartMenuRef>() {
                let _ = menu_ref.0.set_checked(now_enabled);
            }
            tracing::info!(enabled = now_enabled, "autostart toggled");
        }
        Err(e) => {
            tracing::error!(error = %e, "autostart toggle failed");
        }
    }
}
