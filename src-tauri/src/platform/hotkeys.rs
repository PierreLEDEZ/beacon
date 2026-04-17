use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use super::window::NOTCH_LABEL;

pub const SHORTCUT_EVENT: &str = "beacon://shortcut";

#[derive(Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ShortcutAction {
    AllowTopPending,
    DenyTopPending,
}

/// Default visibility shortcut: Ctrl+Alt+Shift+Space.
pub fn default_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::Space,
    )
}

fn allow_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyY,
    )
}

fn deny_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyN,
    )
}

/// Register all Beacon-owned global shortcuts. The visibility toggle is
/// handled entirely in Rust; the Allow/Deny shortcuts emit events for
/// the webview to resolve against its top-of-queue pending — the frontend
/// owns the queue so it's the right place to close the loop.
pub fn register_all(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let toggle = default_shortcut();
    let allow = allow_shortcut();
    let deny = deny_shortcut();

    app.global_shortcut()
        .on_shortcut(toggle, |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_notch_visibility(app);
            }
        })?;

    app.global_shortcut()
        .on_shortcut(allow, |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = app.emit(SHORTCUT_EVENT, ShortcutAction::AllowTopPending);
            }
        })?;

    app.global_shortcut()
        .on_shortcut(deny, |app, _s, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = app.emit(SHORTCUT_EVENT, ShortcutAction::DenyTopPending);
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
