//! Cross-node witness support.

use serde::{Deserialize, Serialize};

use crate::node::NodeId;
use crate::cluster::{ClusterError, Result};
use axiom_core::{Witness, WitnessId};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistributedWitnessStore {
    pub origin_node: Option<NodeId>,
    pub witness_ids: Vec<WitnessId>,
}

impl DistributedWitnessStore {
    pub fn new(origin_node: NodeId) -> Self {
        Self {
            origin_node: Some(origin_node),
            witness_ids: Vec::new(),
        }
    }

    pub fn add(&mut self, witness: &Witness) {
        self.witness_ids.push(witness.witness_id.clone());
    }

    pub fn len(&self) -> usize {
        self.witness_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.witness_ids.is_empty()
    }
}

pub struct WitnessSync {
    _local: NodeId,
}

impl WitnessSync {
    pub fn new(local: NodeId) -> Self {
        Self { _local: local }
    }

    pub fn validate_chain(&self, witnesses: &[Witness]) -> Result<()> {
        if witnesses.is_empty() {
            return Ok(());
        }

        let mut seen = std::collections::HashSet::new();
        for w in witnesses {
            if !seen.insert(w.witness_id.clone()) {
                return Err(ClusterError::SyncError(format!(
                    "duplicate witness id: {}",
                    w.witness_id
                )));
            }
        }

        Ok(())
    }
}
