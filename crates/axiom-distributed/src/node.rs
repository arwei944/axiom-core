//! Node identity and metadata.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 16]);

impl NodeId {
    pub fn new(_id: &str) -> Self {
        let uuid = Uuid::new_v4();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(uuid.as_bytes());
        Self(bytes)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node-{}", Uuid::from_bytes(self.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub address: String,
    pub started_at_ns: u64,
    pub labels: Vec<(String, String)>,
}

impl NodeInfo {
    pub fn new(address: impl Into<String>, labels: Vec<(String, String)>) -> Self {
        let address = address.into();
        let node_id = NodeId::new(&address);
        let started_at_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Self {
            node_id,
            address,
            started_at_ns,
            labels,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Joining,
    Alive,
    Suspect,
    Leaving,
    Dead,
}
