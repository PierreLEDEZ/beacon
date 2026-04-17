//! Pending blocking decisions.
//!
//! For every `PreToolUse` event the server registers a one-shot channel
//! keyed by `event_id`. The hook's `GET /wait/:id` consumes the receiver
//! side; the frontend's `POST /decision/:id` sends on the sender side.
//! A tokio timeout on the receiver auto-resolves to `Deny` when the user
//! doesn't respond in time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// Default timeout before an unanswered prompt auto-denies.
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionKind {
    Allow,
    Deny,
    /// Reserved for AskUserQuestion: the user picked/typed an answer.
    /// Treated as Allow by the hook's stdout translation.
    Answer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub decision: DecisionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
}

impl Decision {
    pub fn timeout_deny() -> Self {
        Self {
            decision: DecisionKind::Deny,
            reason: Some("Beacon timeout: no response within the configured window".into()),
            answer: None,
        }
    }
}

/// Metadata for a pending prompt — everything the UI needs to render.
#[derive(Debug, Clone, Serialize)]
pub struct PendingEvent {
    pub event_id: String,
    pub session_id: String,
    pub event_type: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Server-side pending-decision registry.
#[derive(Clone, Default)]
pub struct PendingDecisions {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    senders: HashMap<String, oneshot::Sender<Decision>>,
    receivers: HashMap<String, oneshot::Receiver<Decision>>,
    meta: HashMap<String, PendingEvent>,
}

impl PendingDecisions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new pending entry; the receiver side is claimed later by
    /// `GET /wait/:id`. Panics if `event_id` already exists (UUID collision
    /// implies we have bigger problems).
    pub fn register(&self, meta: PendingEvent) {
        let (tx, rx) = oneshot::channel();
        let mut inner = self.inner.lock().expect("pending lock poisoned");
        inner.senders.insert(meta.event_id.clone(), tx);
        inner.receivers.insert(meta.event_id.clone(), rx);
        inner.meta.insert(meta.event_id.clone(), meta);
    }

    pub fn take_receiver(&self, event_id: &str) -> Option<oneshot::Receiver<Decision>> {
        self.inner
            .lock()
            .expect("pending lock poisoned")
            .receivers
            .remove(event_id)
    }

    /// Resolve with the given decision. Returns `true` if the event was
    /// actually found and hadn't been resolved yet. Also cleans up metadata.
    pub fn resolve(&self, event_id: &str, decision: Decision) -> bool {
        let mut inner = self.inner.lock().expect("pending lock poisoned");
        let tx = inner.senders.remove(event_id);
        inner.meta.remove(event_id);
        // Note: receiver may have already been taken by /wait; that's fine.
        match tx {
            Some(tx) => tx.send(decision).is_ok(),
            None => false,
        }
    }

    /// Remove metadata + sender after a timeout. The receiver has already
    /// been dropped by the /wait handler when it timed out.
    pub fn drop_meta(&self, event_id: &str) {
        let mut inner = self.inner.lock().expect("pending lock poisoned");
        inner.senders.remove(event_id);
        inner.meta.remove(event_id);
    }

    pub fn list(&self) -> Vec<PendingEvent> {
        let inner = self.inner.lock().expect("pending lock poisoned");
        let mut v: Vec<PendingEvent> = inner.meta.values().cloned().collect();
        v.sort_by_key(|p| p.created_at);
        v
    }

    pub fn has(&self, event_id: &str) -> bool {
        self.inner
            .lock()
            .expect("pending lock poisoned")
            .meta
            .contains_key(event_id)
    }
}

/// Payload for `POST /decision/:id` coming from the frontend.
#[derive(Debug, Deserialize)]
pub struct DecisionInput {
    pub decision: DecisionKind,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub answer: Option<String>,
}

impl From<DecisionInput> for Decision {
    fn from(v: DecisionInput) -> Self {
        Self {
            decision: v.decision,
            reason: v.reason,
            answer: v.answer,
        }
    }
}
