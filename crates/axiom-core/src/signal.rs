//! Signal - Typed immutable message with causal tracking (Vector Clock, correlation).

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Vector Clock for causal ordering.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub std::collections::HashMap<String, u64>);

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter for a given cell.
    pub fn increment(&mut self, cell_id: &str) {
        *self.0.entry(cell_id.to_string()).or_insert(0) += 1;
    }

    /// Merge another vector clock (takes max for each entry).
    pub fn merge(&mut self, other: &VectorClock) {
        for (key, value) in &other.0 {
            let entry = self.0.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    /// Check if this clock causally precedes another (this <= other).
    pub fn causally_precedes(&self, other: &VectorClock) -> bool {
        for (key, &self_val) in &self.0 {
            match other.0.get(key) {
                Some(&other_val) if self_val > other_val => return false,
                None if self_val > 0 => return false,
                _ => {}
            }
        }
        true
    }
}

/// Signal categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    /// Request an operation (mutates state).
    Command,
    /// Notification that something happened (immutable fact).
    Event,
    /// Query state (read-only).
    Query,
}

/// Clone a boxed signal (for dyn compatibility).
pub trait SignalClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn Signal>;
}

impl<T: Signal + Clone> SignalClone for T {
    fn clone_box(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
}

/// Base trait for all signals (dyn-compatible for type-erased message bus).
pub trait Signal: SignalClone + Send + Sync + 'static {
    /// Unique signal type identifier.
    fn signal_type(&self) -> &'static str;

    /// Unique message identifier for idempotency.
    fn msg_id(&self) -> &str;

    /// Correlation ID for distributed tracing.
    fn correlation_id(&self) -> &str;

    /// Vector clock for causal ordering.
    fn vector_clock(&self) -> &VectorClock;

    /// Timestamp (nanoseconds since UNIX epoch) for freshness checks.
    fn timestamp_ns(&self) -> u64;

    /// Signal category.
    fn kind(&self) -> SignalKind;

    /// Sender cell ID, if known.
    fn sender(&self) -> Option<&str> {
        None
    }
}

impl Clone for Box<dyn Signal> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Freshness check: is this signal stale (older than max_age_ns)?
pub fn is_fresh(signal: &dyn Signal, max_age_ns: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    now.saturating_sub(signal.timestamp_ns()) <= max_age_ns
}
