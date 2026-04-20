//! SQLite-backed event history.
//!
//! Every POST /event is mirrored into `%APPDATA%\Beacon\history.db` so
//! users can reach back into past activity without scrolling their
//! terminal transcripts. Designed for low-throughput writes (hundreds
//! per hour at most), so a single Mutex-guarded Connection is plenty.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EventRecord {
    pub id: i64,
    pub event_id: String,
    pub session_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub cwd: String,
    pub created_at: DateTime<Utc>,
    /// Free-form JSON captured from the inbound payload — currently the
    /// tool_input blob so the UI can rebuild a diff / command snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Clone)]
pub struct History {
    conn: Arc<Mutex<Connection>>,
}

impl History {
    /// Open (or create) the database. On any open/migrate failure we log
    /// and return None from `try_open` — history is best-effort: a
    /// broken DB must not prevent Beacon from starting.
    pub fn try_open() -> Option<Self> {
        match Self::open_impl() {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::error!(error = %e, "history: failed to open sqlite");
                None
            }
        }
    }

    fn open_impl() -> Result<Self, rusqlite::Error> {
        let path = history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        // WAL makes single-writer / multi-reader concurrent access
        // cheap and survives app crashes better than the default.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id      TEXT NOT NULL,
                session_id    TEXT NOT NULL,
                event_type    TEXT NOT NULL,
                tool_name     TEXT,
                cwd           TEXT NOT NULL,
                created_at    TEXT NOT NULL,
                metadata_json TEXT
            );
             CREATE INDEX IF NOT EXISTS events_session_idx
                ON events(session_id, id DESC);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Insert a new event. Silent on failure (logged only) — the UI path
    /// must never block on a history write.
    pub fn record(&self, rec: &EventRecord) {
        let conn = match self.conn.lock() {
            Ok(g) => g,
            Err(_) => {
                tracing::error!("history lock poisoned; skipping record");
                return;
            }
        };
        let r = conn.execute(
            "INSERT INTO events (event_id, session_id, event_type, tool_name, cwd, created_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                rec.event_id,
                rec.session_id,
                rec.event_type,
                rec.tool_name,
                rec.cwd,
                rec.created_at.to_rfc3339(),
                rec.metadata,
            ],
        );
        if let Err(e) = r {
            tracing::warn!(error = %e, "history insert failed");
        }
    }

    pub fn list_for_session(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<EventRecord>, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::ToSqlConversionFailure("history lock poisoned".into())
        })?;
        let mut stmt = conn.prepare(
            "SELECT id, event_id, session_id, event_type, tool_name, cwd, created_at, metadata_json
             FROM events
             WHERE session_id = ?1
             ORDER BY id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, limit], |row| {
            let created: String = row.get(6)?;
            let created = DateTime::parse_from_rfc3339(&created)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(EventRecord {
                id: row.get(0)?,
                event_id: row.get(1)?,
                session_id: row.get(2)?,
                event_type: row.get(3)?,
                tool_name: row.get(4)?,
                cwd: row.get(5)?,
                created_at: created,
                metadata: row.get(7)?,
            })
        })?;
        rows.collect()
    }
}

fn history_path() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("Beacon").join("history.db"))
        .unwrap_or_else(|| PathBuf::from("beacon-history.db"))
}
