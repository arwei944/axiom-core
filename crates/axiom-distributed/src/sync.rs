//! Minimum event synchronization across nodes.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cluster::Result;
use crate::node::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub source: NodeId,
    pub target: NodeId,
    pub from_sequence: u64,
    pub to_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub source: NodeId,
    pub target: NodeId,
    pub events: Vec<SyncEvent>,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub sequence_number: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Default, Clone)]
pub struct SyncState {
    pub last_acked_sequence: u64,
}

pub struct EventSync {
    _local: NodeId,
    state: Arc<RwLock<SyncState>>,
}

impl EventSync {
    pub fn new(local: NodeId) -> Self {
        Self { _local: local, state: Arc::new(RwLock::new(SyncState::default())) }
    }

    pub async fn sync(&self, request: SyncRequest) -> Result<SyncResponse> {
        let state = self.state.read().await;
        let events = Vec::new();
        Ok(SyncResponse {
            source: request.source,
            target: request.target,
            events,
            last_sequence: state.last_acked_sequence,
        })
    }

    pub async fn apply(&self, response: SyncResponse) -> Result<()> {
        let mut state = self.state.write().await;
        if response.last_sequence > state.last_acked_sequence {
            state.last_acked_sequence = response.last_sequence;
        }
        Ok(())
    }
}
