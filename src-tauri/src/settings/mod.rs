//! User settings persisted to `%APPDATA%\Beacon\settings.json`.
//!
//! Settings are loaded once at startup and shared via Tauri's managed
//! state. `Settings::apply_and_save` mutates + rewrites the file in a
//! single shot — the frontend talks to the backend via two Tauri
//! commands (see lib.rs): `get_settings` and `update_settings`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 37421;
pub const DEFAULT_DISTRO: &str = "Ubuntu";
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotchMonitor {
    /// Pick the monitor under the cursor at each fresh launch.
    Cursor,
    /// Always place on the OS primary monitor.
    Primary,
}

impl Default for NotchMonitor {
    fn default() -> Self {
        NotchMonitor::Cursor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_distro")]
    pub wsl_distro: String,
    #[serde(default = "default_timeout")]
    pub decision_timeout_secs: u64,
    #[serde(default)]
    pub notch_monitor: NotchMonitor,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}
fn default_distro() -> String {
    DEFAULT_DISTRO.into()
}
fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            wsl_distro: DEFAULT_DISTRO.into(),
            decision_timeout_secs: DEFAULT_TIMEOUT_SECS,
            notch_monitor: NotchMonitor::default(),
        }
    }
}

/// Thread-safe handle to the live settings. Mutations go through
/// `update` so the write is atomic (in-memory mutation + disk flush).
#[derive(Clone)]
pub struct SettingsStore {
    inner: Arc<Mutex<Settings>>,
    path: PathBuf,
}

impl SettingsStore {
    pub fn load_or_default() -> Self {
        let path = settings_path();
        let settings = match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Settings>(&raw) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "settings parse failed, using defaults");
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        };
        Self {
            inner: Arc::new(Mutex::new(settings)),
            path,
        }
    }

    pub fn get(&self) -> Settings {
        self.inner.lock().expect("settings lock poisoned").clone()
    }

    /// Apply the given settings and persist. Returns the stored value.
    pub fn update(&self, next: Settings) -> Result<Settings, String> {
        {
            let mut guard = self.inner.lock().expect("settings lock poisoned");
            *guard = next.clone();
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir settings dir: {e}"))?;
        }
        let serialized = serde_json::to_string_pretty(&next)
            .map_err(|e| format!("serialize settings: {e}"))?;
        std::fs::write(&self.path, serialized)
            .map_err(|e| format!("write settings: {e}"))?;
        Ok(next)
    }
}

fn settings_path() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("Beacon").join("settings.json"))
        // As a dev-environment fallback, keep settings next to the cwd
        // rather than panicking if %APPDATA% lookup fails.
        .unwrap_or_else(|| PathBuf::from("beacon-settings.json"))
}
