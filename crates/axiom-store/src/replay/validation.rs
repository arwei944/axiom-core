//! Validation and diff utilities for replay.

use crate::replay::{ReplayResult, ReplayableState};
use crate::store::{EventStore, StoreError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupValidation {
    pub validated_types: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff<S> {
    pub from_state: S,
    pub to_state: S,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub from_timestamp_ns: u64,
    pub to_timestamp_ns: u64,
    pub changed_fields: Vec<String>,
}

impl<S: ReplayableState + Clone> StateDiff<S> {
    pub fn compute(from: ReplayResult<S>, to: ReplayResult<S>) -> Self {
        let mut changed_fields = Vec::new();
        if from.state.to_snapshot() != to.state.to_snapshot() {
            changed_fields.push("state changed".to_string());
        }

        Self {
            from_state: from.state,
            to_state: to.state,
            from_sequence: from.last_sequence,
            to_sequence: to.last_sequence,
            from_timestamp_ns: 0,
            to_timestamp_ns: 0,
            changed_fields,
        }
    }
}

pub async fn validate_migration_chains_at_startup(
    _store: &dyn EventStore,
) -> Result<StartupValidation, StoreError> {
    Ok(StartupValidation {
        validated_types: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}
