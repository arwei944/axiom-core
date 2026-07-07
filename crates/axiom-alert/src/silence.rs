//! Silence and suppression.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Silence {
    pub id: String,
    pub matchers: Vec<(String, String)>,
    pub starts_at_ns: u64,
    pub ends_at_ns: u64,
    pub created_by: String,
    pub comment: String,
}

impl Silence {
    pub fn new(
        matchers: Vec<(String, String)>,
        ends_at_ns: u64,
        created_by: &str,
        comment: &str,
    ) -> Self {
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        Self {
            id: format!("silence-{}", Uuid::new_v4()),
            matchers,
            starts_at_ns: now,
            ends_at_ns,
            created_by: created_by.to_string(),
            comment: comment.to_string(),
        }
    }

    pub fn matches(&self, alert: &crate::alert::Alert) -> bool {
        self.starts_at_ns <= alert.starts_at_ns
            && (self.ends_at_ns == 0 || alert.starts_at_ns <= self.ends_at_ns)
            && self
                .matchers
                .iter()
                .all(|(k, v)| alert.labels.iter().any(|(ak, av)| ak == k && av == v))
    }
}
