//! Dead Letter Queue - captures undeliverable messages for analysis.

use axiom_core::signal::SignalEnvelope;
use std::collections::VecDeque;
use std::sync::RwLock;

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
        Self {
            letters: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn enqueue(&self, envelope: SignalEnvelope, reason: &str) {
        let mut letters = self.letters.write().unwrap();
        if letters.len() >= self.capacity {
            letters.pop_front();
        }
        letters.push_back(DeadLetter {
            envelope,
            reason: reason.to_string(),
            timestamp_ns: axiom_core::signal::now_ns(),
        });
    }

    pub fn len(&self) -> usize {
        self.letters.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn drain(&self) -> Vec<DeadLetter> {
        let mut letters = self.letters.write().unwrap();
        letters.drain(..).collect()
    }

    pub fn peek_all(&self) -> Vec<DeadLetter> {
        self.letters.read().unwrap().iter().cloned().collect()
    }
}

impl Default for DeadLetterQueue {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_core::id::{CorrelationId, MsgId};
    use axiom_core::layer::Layer;
    use axiom_core::signal::{SignalKind, VectorClock};

    fn make_env() -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("dlq-test"),
            correlation_id: CorrelationId::new("dlq-corr"),
            trace_id: None,
            signal_type: "Test".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: Layer::Exec,
            target_layer: Layer::Exec,
            source_cell: None,
            target_cell: Some("c1".to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_core::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn test_dlq_enqueue_and_drain() {
        let dlq = DeadLetterQueue::new(10);
        dlq.enqueue(make_env(), "mailbox full");
        dlq.enqueue(make_env(), "target not found");
        assert_eq!(dlq.len(), 2);
        let drained = dlq.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].reason, "mailbox full");
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_dlq_capacity_evicts_oldest() {
        let dlq = DeadLetterQueue::new(3);
        for i in 0..5 {
            let mut env = make_env();
            env.msg_id = MsgId::new(format!("m{}", i));
            dlq.enqueue(env, &format!("reason{}", i));
        }
        assert_eq!(dlq.len(), 3);
        let all = dlq.peek_all();
        assert_eq!(all[0].reason, "reason2");
        assert_eq!(all[2].reason, "reason4");
    }
}
