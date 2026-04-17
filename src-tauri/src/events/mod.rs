use serde::Serialize;
use tokio::sync::broadcast;

use crate::session::Session;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BusMessage {
    SessionUpdated { session: Session },
    SessionRemoved { claude_session_id: String },
}

/// Small wrapper around a tokio broadcast channel so backend components stay
/// decoupled from the frontend emitter. Channel capacity is intentionally
/// modest — if a subscriber lags, broadcast drops the oldest messages rather
/// than applying back-pressure, which is exactly what we want for UI updates.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<BusMessage>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, msg: BusMessage) {
        // Ignore send errors: they only occur when there are zero receivers,
        // which is fine — the frontend may simply not be listening yet.
        let _ = self.tx.send(msg);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
