//! Mailbox - per-Cell concurrent message queue with capacity control (P2-1).
//! Uses parking_lot mutex for lower latency than tokio Mutex on short critical sections.

use std::collections::VecDeque;
use tokio::sync::Semaphore;

use axiom_kernel::signal::SignalEnvelope;

pub struct Mailbox {
    queue: parking_lot::Mutex<VecDeque<SignalEnvelope>>,
    capacity: usize,
    permits: Semaphore,
}

impl Mailbox {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: parking_lot::Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            permits: Semaphore::new(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub async fn len(&self) -> usize {
        self.queue.lock().len()
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    pub async fn push(&self, env: SignalEnvelope) -> Result<(), SignalEnvelope> {
        let permit = match self.permits.try_acquire() {
            Ok(p) => p,
            Err(_) => return Err(env),
        };
        self.queue.lock().push_back(env);
        std::mem::forget(permit);
        Ok(())
    }

    pub async fn pop(&self) -> Option<SignalEnvelope> {
        let mut q = self.queue.lock();
        let env = q.pop_front();
        if env.is_some() {
            self.permits.add_permits(1);
        }
        env
    }

    pub async fn drain(&self) -> Vec<SignalEnvelope> {
        let mut q = self.queue.lock();
        let count = q.len();
        let drained: Vec<SignalEnvelope> = q.drain(..).collect();
        self.permits.add_permits(count);
        drained
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mailbox_push_pop() {
        let mb = Mailbox::new(8);
        use axiom_kernel::id::{CorrelationId, MsgId};
        use axiom_kernel::layer::RuntimeTier;
        use axiom_kernel::signal::{SignalKind, VectorClock};

        let env = SignalEnvelope {
            msg_id: MsgId::new("m1"),
            correlation_id: CorrelationId::new("c1"),
            trace_id: None,
            signal_type: "test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: RuntimeTier::Exec,
            target_layer: RuntimeTier::Exec,
            source_cell: None,
            target_cell: Some("cell-a".into()),
            payload: serde_json::Value::Null,
            schema_version: axiom_kernel::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        };

        mb.push(env.clone()).await.unwrap();
        assert_eq!(mb.len().await, 1);
        let got = mb.pop().await.unwrap();
        assert_eq!(got.msg_id.as_str(), "m1");
        assert!(mb.is_empty().await);
    }

    #[tokio::test]
    async fn test_mailbox_capacity_reject() {
        let mb = Mailbox::new(1);
        use axiom_kernel::id::{CorrelationId, MsgId};
        use axiom_kernel::layer::RuntimeTier;
        use axiom_kernel::signal::{SignalKind, VectorClock};

        let make_env = |id: &str| SignalEnvelope {
            msg_id: MsgId::new(id),
            correlation_id: CorrelationId::new("c1"),
            trace_id: None,
            signal_type: "test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: RuntimeTier::Exec,
            target_layer: RuntimeTier::Exec,
            source_cell: None,
            target_cell: Some("cell-a".into()),
            payload: serde_json::Value::Null,
            schema_version: axiom_kernel::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        };

        mb.push(make_env("m1")).await.unwrap();
        let result = mb.push(make_env("m2")).await;
        assert!(result.is_err(), "second push should be rejected at capacity 1");
    }
}
