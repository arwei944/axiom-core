//! Dead Letter Queue - captures undeliverable messages for analysis.

use crate::constants::DEFAULT_DLQ_CAPACITY;
use axiom_kernel::clock::global_clock;
use axiom_kernel::signal::SignalEnvelope;
use axiom_kernel::KernelError;
use parking_lot::RwLock;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct DeadLetter {
    pub envelope: SignalEnvelope,
    pub reason: String,
    pub timestamp_ns: u64,
}

pub struct DeadLetterQueue {
    letters: RwLock<VecDeque<DeadLetter>>,
    capacity: usize,
}

impl DeadLetterQueue {
    pub fn new(capacity: usize) -> Self {
        Self { letters: RwLock::new(VecDeque::with_capacity(capacity)), capacity }
    }

    pub fn enqueue(&self, envelope: SignalEnvelope, reason: &str) -> Result<(), KernelError> {
        let mut letters = self.letters.write();
        if letters.len() >= self.capacity {
            return Err(KernelError::ResourceExhausted {
                resource: format!("dlq capacity {} exceeded", self.capacity),
                cell_id: "dlq".to_string(),
            });
        }
        letters.push_back(DeadLetter {
            envelope,
            reason: reason.to_string(),
            timestamp_ns: global_clock().now_ns(),
        });
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.letters.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn drain(&self) -> Vec<DeadLetter> {
        let mut letters = self.letters.write();
        letters.drain(..).collect()
    }

    pub fn peek_all(&self) -> Vec<DeadLetter> {
        self.letters.read().iter().cloned().collect()
    }
}

impl Default for DeadLetterQueue {
    fn default() -> Self {
        Self::new(DEFAULT_DLQ_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::layer::RuntimeTier;
    use axiom_kernel::signal::{SignalKind, VectorClock};

    fn make_env() -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("dlq-test"),
            correlation_id: CorrelationId::new("dlq-corr"),
            trace_id: None,
            signal_type: "Test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: RuntimeTier::Exec,
            target_layer: RuntimeTier::Exec,
            source_cell: None,
            target_cell: Some("c1".to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_kernel::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn test_dlq_enqueue_and_drain() {
        let dlq = DeadLetterQueue::new(10);
        dlq.enqueue(make_env(), "mailbox full").unwrap();
        dlq.enqueue(make_env(), "target not found").unwrap();
        assert_eq!(dlq.len(), 2);
        let drained = dlq.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].reason, "mailbox full");
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_dlq_capacity_returns_error_when_full() {
        let dlq = DeadLetterQueue::new(3);
        for i in 0..3 {
            let mut env = make_env();
            env.msg_id = MsgId::new(format!("m{}", i));
            dlq.enqueue(env, &format!("reason{}", i)).unwrap();
        }
        assert_eq!(dlq.len(), 3);
        let mut env = make_env();
        env.msg_id = MsgId::new("m-full");
        let result = dlq.enqueue(env, "overflow");
        assert!(result.is_err());
        assert_eq!(dlq.len(), 3);
    }
}
