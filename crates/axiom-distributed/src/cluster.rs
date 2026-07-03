//! Cluster view and configuration.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::node::{NodeId, NodeInfo, NodeState};

#[derive(Debug, Error)]
pub enum ClusterError {
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("sync error: {0}")]
    SyncError(String),
}

pub type Result<T, E = ClusterError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub node_id: NodeId,
    pub seeds: Vec<String>,
    pub gossip_interval_ms: u64,
    pub suspect_timeout_ms: u64,
    pub dead_timeout_ms: u64,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId::new("default"),
            seeds: Vec::new(),
            gossip_interval_ms: 1000,
            suspect_timeout_ms: 5000,
            dead_timeout_ms: 15000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterView {
    pub nodes: Vec<(NodeId, NodeInfo, NodeState)>,
}

impl ClusterView {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn contains(&self, node_id: NodeId) -> bool {
        self.nodes.iter().any(|(id, _, _)| *id == node_id)
    }
}
