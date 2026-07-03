//! Axiom Distributed - distributed primitives for Axiom.
//!
//! Provides:
//! - Node / Cluster model
//! - Node discovery
//! - Event synchronization
//! - Cross-node witness

pub mod cluster;
pub mod discovery;
pub mod node;
pub mod sync;
pub mod witness;

pub use cluster::{ClusterConfig, ClusterError, ClusterView};
pub use discovery::{DiscoveryConfig, DiscoveryEvent, NodeDiscovery};
pub use node::{NodeId, NodeInfo, NodeState};
pub use sync::{SyncRequest, SyncResponse, SyncState};
pub use witness::{DistributedWitnessStore, WitnessSync};
