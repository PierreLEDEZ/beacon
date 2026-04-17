use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use super::window::NOTCH_LABEL;

/// Default global shortcut: Ctrl+Alt+Shift+Space.
pub fn default_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::Space,
    )
}

/// Register the default global shortcut. The handler toggles the notch
/// window's visibility so users can hide Beacon out of the way and recall
/// it without reaching for the tray.
pub fn register_toggle_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = default_shortcut();
    app.global_shortcut().on_shortcut(shortcut, |app, _s, event| {
        if event.state() == ShortcutState::Pressed {
            toggle_notch_visibility(app);
        }
    })?;
    Ok(())
}

fn toggle_notch_visibility(app: &AppHandle) {
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
