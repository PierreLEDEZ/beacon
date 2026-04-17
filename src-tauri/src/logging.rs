//! Tracing setup: every log line goes to `%APPDATA%\Beacon\logs\beacon.log`
//! with daily rotation, and also to stderr while in debug builds.

use std::fs;
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize global tracing. Returns a guard that MUST be held for the
/// lifetime of the app — dropping it synchronously flushes buffered logs
/// and shuts down the writer thread. Returning None means logging is
/// disabled (e.g. the log dir could not be created); the app continues.
pub fn init() -> Option<WorkerGuard> {
    let log_dir = log_dir()?;
    fs::create_dir_all(&log_dir).ok()?;

    let appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "beacon.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("BEACON_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer);

    // In debug builds, also mirror to stderr so `cargo tauri dev` shows logs.
    #[cfg(debug_assertions)]
    let registry = registry.with(tracing_subscriber::fmt::layer().with_ansi(true));

    registry.init();
    Some(guard)
}

fn log_dir() -> Option<PathBuf> {
    // dirs::data_dir() = %APPDATA%\Roaming on Windows, ~/.local/share on Linux.
    Some(dirs::data_dir()?.join("Beacon").join("logs"))
}
