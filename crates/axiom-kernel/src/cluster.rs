//! Distributed runtime foundation for multi-node clusters.
//!
//! This module provides:
//! - `ClusterNodeId`: unique identifier for each node in a cluster
//! - `ClusterTopology`: view of the cluster (local node + remote peers)
//! - `ClusterTransport`: abstraction for sending messages between nodes
//! - `ReplicatedCellKernel`: CellKernel that can forward `send/receive` to remote nodes
//!
//! Design goals:
//! - Keep local path fast: local cells use existing `CellKernel` unchanged
//! - Remote path is best-effort async: `ClusterTransport::send_to_node`
//! - No global lock: topology is read-heavy, uses `Arc<RwLock<>>`

use crate::axiom::{KernelError, KernelResult, Message};
use crate::cell::{CellHandle, CellKernel, CellKind};
use crate::id::CellId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use parking_lot::RwLock;

// ============================================================
// Cluster identity & topology
// ============================================================

/// Unique identifier for a cluster node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterNodeId(pub u64);

impl ClusterNodeId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for ClusterNodeId {
    fn default() -> Self {
        Self(0)
    }
}

impl fmt::Display for ClusterNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node-{}", self.0)
    }
}

/// Address information for reaching a node.
///
/// In production this would be a gRPC endpoint; for now we keep a
/// generic address string so the transport layer can interpret it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddress {
    pub host: String,
    pub port: u16,
    pub scheme: String,
}

impl NodeAddress {
    pub fn new(host: impl Into<String>, port: u16, scheme: impl Into<String>) -> Self {
        Self { host: host.into(), port, scheme: scheme.into() }
    }

    pub fn to_url(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.port)
    }
}

/// Lightweight view of the cluster from the perspective of one node.
#[derive(Debug, Clone, Default)]
pub struct ClusterTopology {
    /// The node this process owns.
    pub local_node: ClusterNodeId,
    /// Known peer nodes and how to reach them.
    pub peers: HashMap<ClusterNodeId, NodeAddress>,
    /// Mapping of `cell_id -> owner_node`. Local cells may be omitted.
    pub cell_placement: HashMap<CellId, ClusterNodeId>,
}

impl ClusterTopology {
    pub fn new(local_node: ClusterNodeId) -> Self {
        Self {
            local_node,
            peers: HashMap::new(),
            cell_placement: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, node_id: ClusterNodeId, address: NodeAddress) {
        self.peers.insert(node_id, address);
    }

    pub fn place_cell(&mut self, cell_id: CellId, node_id: ClusterNodeId) {
        self.cell_placement.insert(cell_id, node_id);
    }

    pub fn owner_of(&self, cell_id: &CellId) -> Option<ClusterNodeId> {
        self.cell_placement.get(cell_id).copied()
    }

    pub fn is_local(&self, cell_id: &CellId) -> bool {
        self.owner_of(cell_id).map_or(true, |owner| owner == self.local_node)
    }
}

// ============================================================
// Transport abstraction
// ============================================================

/// Outcome of a remote send attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    /// Message accepted by the remote node.
    Accepted,
    /// Remote node reached, but it rejected the message.
    Rejected,
    /// Transport-level failure (network, timeout, etc.).
    TransportFailed,
}

/// Abstraction for shipping messages between cluster nodes.
///
/// Implementations:
/// - `InMemoryClusterTransport`: loopback within the same process
/// - `GrpcClusterTransport`: real network transport (future)
pub trait ClusterTransport: Send + Sync {
    /// Send a message to a specific remote node.
    ///
    /// The transport is responsible for routing to the correct cell
    /// on the remote side, or at least delivering it to the remote
    /// node's inbound queue.
    fn send_to_node(
        &self,
        from_node: ClusterNodeId,
        to_node: ClusterNodeId,
        envelope: crate::signal::SignalEnvelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = SendOutcome> + Send + '_>>;

    /// Best-effort broadcast to all known peers.
    ///
    /// Used for topology changes, gossip, etc.
    fn broadcast(
        &self,
        _from_node: ClusterNodeId,
        _envelope: crate::signal::SignalEnvelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            // Default: no-op. Real implementations iterate peers.
        })
    }
}

/// In-memory transport useful for tests and single-process clusters.
pub struct InMemoryClusterTransport {
    /// Shared inboxes keyed by node id.
    inboxes: std::sync::Arc<tokio::sync::RwLock<HashMap<ClusterNodeId, Vec<crate::signal::SignalEnvelope>>>>,
}

impl InMemoryClusterTransport {
    pub fn new() -> Self {
        Self { inboxes: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())) }
    }

    pub async fn drain_inbox(&self, node_id: ClusterNodeId) -> Vec<crate::signal::SignalEnvelope> {
        let mut inboxes = self.inboxes.write().await;
        inboxes.remove(&node_id).unwrap_or_default()
    }
}

impl Default for InMemoryClusterTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterTransport for InMemoryClusterTransport {
    fn send_to_node(
        &self,
        _from_node: ClusterNodeId,
        to_node: ClusterNodeId,
        envelope: crate::signal::SignalEnvelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = SendOutcome> + Send + '_>> {
        Box::pin(async move {
            let mut inboxes = self.inboxes.write().await;
            inboxes.entry(to_node).or_default().push(envelope);
            SendOutcome::Accepted
        })
    }
}

// ============================================================
// Replicated CellKernel
// ============================================================

/// `CellKernel` extension that can forward messages to cells on remote nodes.
///
/// Local cells are stored in the embedded `CellKernel`. Remote cells are
/// tracked by placement only; their actual state lives on the owner node.
pub struct ReplicatedCellKernel {
    local: CellKernel,
    topology: Arc<RwLock<ClusterTopology>>,
    transport: Arc<dyn ClusterTransport>,
}

impl ReplicatedCellKernel {
    pub fn new(
        local_node: ClusterNodeId,
        transport: Arc<dyn ClusterTransport>,
    ) -> Self {
        Self {
            local: CellKernel::new(),
            topology: Arc::new(RwLock::new(ClusterTopology::new(local_node))),
            transport,
        }
    }

    pub fn with_heatmap(
        local_node: ClusterNodeId,
        transport: Arc<dyn ClusterTransport>,
        heatmap: std::sync::Arc<parking_lot::RwLock<crate::HeatmapCollector>>,
    ) -> Self {
        Self {
            local: CellKernel::with_heatmap(heatmap),
            topology: Arc::new(RwLock::new(ClusterTopology::new(local_node))),
            transport,
        }
    }

    pub fn local_kernel(&self) -> &CellKernel {
        &self.local
    }

    pub async fn topology(&self) -> ClusterTopology {
        self.topology.read().await.clone()
    }

    pub async fn add_peer(&self, node_id: ClusterNodeId, address: NodeAddress) {
        self.topology.write().await.add_peer(node_id, address);
    }

    pub async fn place_cell(&self, cell_id: CellId, node_id: ClusterNodeId) {
        self.topology.write().await.place_cell(cell_id, node_id);
    }

    /// Send a message to a cell, routing remotely if necessary.
    pub async fn send(&self, handle: &CellHandle, msg: Message) -> KernelResult<()> {
        let topology = self.topology.read().await;
        if topology.is_local(&handle.id) {
            drop(topology);
            self.local.send(handle, msg).await
        } else {
            let owner = topology.owner_of(&handle.id).ok_or_else(|| {
                KernelError::CellNotFound(format!("{} has no owner in cluster", handle.id))
            })?;
            drop(topology);

            let mut envelope = crate::signal::SignalEnvelope::new(
                crate::RuntimeTier::Exec,
                crate::RuntimeTier::Exec,
                "remote_message",
            );
            envelope.target_cell = Some(handle.id.to_string());
            envelope.payload = serde_json::to_value(&msg).unwrap_or(serde_json::Value::Null);

            let local_node = self.topology.read().await.local_node;
            let outcome = self
                .transport
                .send_to_node(local_node, owner, envelope)
                .await;

            match outcome {
                SendOutcome::Accepted => Ok(()),
                SendOutcome::Rejected => Err(KernelError::CellNotFound(format!(
                    "remote node rejected cell {}",
                    handle.id
                ))),
                SendOutcome::TransportFailed => Err(KernelError::CellNotFound(format!(
                    "transport failed for cell {}",
                    handle.id
                ))),
            }
        }
    }

    pub async fn receive(&self, handle: &CellHandle) -> KernelResult<Option<Message>> {
        let topology = self.topology.read().await;
        if topology.is_local(&handle.id) {
            self.local.receive(handle).await
        } else {
            Err(KernelError::CellNotFound(format!(
                "{} is not local; receive must be performed on owner node",
                handle.id
            )))
        }
    }

    pub async fn create(&self, kind: CellKind) -> CellHandle {
        self.local.create(kind).await
    }

    pub async fn count(&self) -> usize {
        self.local.count().await
    }

    pub async fn list(&self) -> Vec<(CellHandle, usize)> {
        self.local.list().await
    }

    pub async fn status(&self) -> Vec<crate::cell::CellStatus> {
        self.local.status().await
    }
}

// ============================================================
// State synchronization
// ============================================================

/// Serializable snapshot of cell state for cross-node transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    pub cell_id: CellId,
    pub owner_node: ClusterNodeId,
    pub state: serde_json::Value,
    pub vector_clock: crate::signal::VectorClock,
    pub updated_at_ns: u64,
    pub schema_version: crate::version::SchemaVersion,
}

/// Conflict resolution strategy for divergent cell states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Latest update wins; ties broken by comparing vector clocks.
    LastWriterWins,
    /// Keep local state (useful for manual reconciliation).
    PreferLocal,
    /// Keep remote state (useful for authoritative nodes).
    PreferRemote,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::LastWriterWins
    }
}

/// Protocol helper for merging state across nodes.
///
/// Currently provides:
/// - vector-clock merge
/// - last-writer-wins conflict resolution
/// - snapshot comparison helpers
pub struct StateSyncProtocol {
    conflict_resolution: ConflictResolution,
}

impl StateSyncProtocol {
    pub fn new(conflict_resolution: ConflictResolution) -> Self {
        Self { conflict_resolution }
    }

    pub fn default_clock_merge(local: &crate::signal::VectorClock, remote: &crate::signal::VectorClock) -> crate::signal::VectorClock {
        let mut merged = local.clone();
        merged.merge(remote);
        merged
    }

    pub fn resolve_conflict(
        &self,
        local_snapshot: &SyncSnapshot,
        remote_snapshot: &SyncSnapshot,
    ) -> SyncSnapshot {
        match self.conflict_resolution {
            ConflictResolution::LastWriterWins => {
                if remote_snapshot.updated_at_ns > local_snapshot.updated_at_ns {
                    remote_snapshot.clone()
                } else if local_snapshot.updated_at_ns > remote_snapshot.updated_at_ns {
                    local_snapshot.clone()
                } else {
                    // Tiebreak by vector-clock comparison: the clock with a higher
                    // count for the cell itself wins.
                    let local_count = local_snapshot.vector_clock.get(&local_snapshot.cell_id.to_string());
                    let remote_count = remote_snapshot.vector_clock.get(&remote_snapshot.cell_id.to_string());
                    if remote_count >= local_count {
                        remote_snapshot.clone()
                    } else {
                        local_snapshot.clone()
                    }
                }
            }
            ConflictResolution::PreferLocal => local_snapshot.clone(),
            ConflictResolution::PreferRemote => remote_snapshot.clone(),
        }
    }
}

impl Default for StateSyncProtocol {
    fn default() -> Self {
        Self::new(ConflictResolution::LastWriterWins)
    }
}


