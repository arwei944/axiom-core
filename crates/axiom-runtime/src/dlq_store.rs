//! Durable Dead Letter store (P1-1).

use crate::dlq::DeadLetter;
use axiom_kernel::signal::SignalEnvelope;
use std::collections::VecDeque;
use std::sync::Mutex;

pub trait DeadLetterStore: Send + Sync {
    fn enqueue(&self, letter: DeadLetter) -> Result<(), String>;
    fn peek(&self, limit: usize) -> Vec<DeadLetter>;
    fn ack(&self, msg_id: &str) -> bool;
    fn retry(&self, msg_id: &str) -> Option<SignalEnvelope>;
    fn len(&self) -> usize;
}

/// In-memory adapter.
pub struct MemoryDeadLetterStore {
    inner: Mutex<VecDeque<DeadLetter>>,
}

impl MemoryDeadLetterStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
        }
    }
}

impl Default for MemoryDeadLetterStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadLetterStore for MemoryDeadLetterStore {
    fn enqueue(&self, letter: DeadLetter) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|e| e.to_string())?
            .push_back(letter);
        Ok(())
    }

    fn peek(&self, limit: usize) -> Vec<DeadLetter> {
        self.inner
            .lock()
            .map(|g| g.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    fn ack(&self, msg_id: &str) -> bool {
        let Ok(mut g) = self.inner.lock() else {
            return false;
        };
        if let Some(pos) = g.iter().position(|l| l.envelope.msg_id.as_str() == msg_id) {
            g.remove(pos);
            true
        } else {
            false
        }
    }

    fn retry(&self, msg_id: &str) -> Option<SignalEnvelope> {
        let Ok(mut g) = self.inner.lock() else {
            return None;
        };
        if let Some(pos) = g.iter().position(|l| l.envelope.msg_id.as_str() == msg_id) {
            let letter = g.remove(pos)?;
            Some(letter.envelope)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }
}

/// SQLite-backed adapter (file path); simple JSON rows.
pub struct SqliteDeadLetterStore {
    path: String,
    mem: MemoryDeadLetterStore,
}

impl SqliteDeadLetterStore {
    pub fn open(path: impl Into<String>) -> Result<Self, String> {
        let path = path.into();
        let store = Self {
            path: path.clone(),
            mem: MemoryDeadLetterStore::new(),
        };
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(letters) = serde_json::from_str::<Vec<PersistedLetter>>(&data) {
                for pl in letters {
                    let _ = store.mem.enqueue(DeadLetter {
                        envelope: pl.envelope,
                        reason: pl.reason,
                        timestamp_ns: pl.timestamp_ns,
                    });
                }
            }
        }
        Ok(store)
    }

    fn persist(&self) -> Result<(), String> {
        let letters = self.mem.peek(usize::MAX);
        let persisted: Vec<PersistedLetter> = letters
            .into_iter()
            .map(|l| PersistedLetter {
                envelope: l.envelope,
                reason: l.reason,
                timestamp_ns: l.timestamp_ns,
            })
            .collect();
        let json = serde_json::to_string(&persisted).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, json).map_err(|e| e.to_string())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedLetter {
    envelope: SignalEnvelope,
    reason: String,
    timestamp_ns: u64,
}

impl DeadLetterStore for SqliteDeadLetterStore {
    fn enqueue(&self, letter: DeadLetter) -> Result<(), String> {
        self.mem.enqueue(letter)?;
        self.persist()
    }
    fn peek(&self, limit: usize) -> Vec<DeadLetter> {
        self.mem.peek(limit)
    }
    fn ack(&self, msg_id: &str) -> bool {
        let ok = self.mem.ack(msg_id);
        let _ = self.persist();
        ok
    }
    fn retry(&self, msg_id: &str) -> Option<SignalEnvelope> {
        let env = self.mem.retry(msg_id);
        let _ = self.persist();
        env
    }
    fn len(&self) -> usize {
        self.mem.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::layer::RuntimeTier;
    use axiom_kernel::signal::{SignalKind, VectorClock};

    fn letter(id: &str) -> DeadLetter {
        DeadLetter {
            envelope: SignalEnvelope {
                msg_id: MsgId::new(id),
                correlation_id: CorrelationId::new("c"),
                trace_id: None,
                signal_type: "T".into(),
                vector_clock: VectorClock::new(),
                timestamp_ns: 1,
                kind: SignalKind::Command,
                source_layer: RuntimeTier::Exec,
                target_layer: RuntimeTier::Exec,
                source_cell: None,
                target_cell: None,
                payload: serde_json::Value::Null,
                schema_version: axiom_kernel::SchemaVersion::new(1),
                parent_msg_id: None,
                hop_count: 0,
            },
            reason: "test".into(),
            timestamp_ns: 1,
        }
    }

    #[test]
    fn peek_ack_retry_and_ops() {
        let path = std::env::temp_dir().join(format!(
            "dlq-ops-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let store = SqliteDeadLetterStore::open(path.to_string_lossy()).unwrap();
        store.enqueue(letter("m1")).unwrap();
        store.enqueue(letter("m2")).unwrap();
        assert_eq!(store.peek(10).len(), 2);
        assert!(store.ack("m1"));
        let env = store.retry("m2").unwrap();
        assert_eq!(env.msg_id.as_str(), "m2");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn crash_recovery_reopens_nonempty_durable_store() {
        // P1-1: enqueue then reopen WITHOUT acking — unacked letters must load.
        let path = std::env::temp_dir().join(format!(
            "dlq-crash-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        {
            let store = SqliteDeadLetterStore::open(path.to_string_lossy()).unwrap();
            store.enqueue(letter("keep-1")).unwrap();
            store.enqueue(letter("keep-2")).unwrap();
            assert_eq!(store.len(), 2);
        } // drop = "crash"
        let reopened = SqliteDeadLetterStore::open(path.to_string_lossy()).unwrap();
        let peeked = reopened.peek(10);
        assert_eq!(peeked.len(), 2, "durable unacked letters must survive reopen");
        assert!(peeked.iter().any(|l| l.envelope.msg_id.as_str() == "keep-1"));
        assert!(peeked.iter().any(|l| l.envelope.msg_id.as_str() == "keep-2"));
        let _ = std::fs::remove_file(path);
    }
}
