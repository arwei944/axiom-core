use axiom_kernel::cell::{CellHandle, CellKind};
use axiom_kernel::cluster::{
    ClusterNodeId, ClusterTopology, ConflictResolution, InMemoryClusterTransport, NodeAddress,
    ReplicatedCellKernel, StateSyncProtocol, SyncSnapshot,
};
use axiom_kernel::Message;
use std::sync::Arc;

#[tokio::test]
async fn cluster_topology_local_cell_is_local() {
    let local = ClusterNodeId::new(1);
    let topology = ClusterTopology::new(local);

    let cell_id = axiom_kernel::CellId::new("local-cell".to_string());
    assert!(topology.is_local(&cell_id));
}

#[tokio::test]
async fn replicated_cell_kernel_local_send_uses_local_path() {
    let transport = Arc::new(InMemoryClusterTransport::new());
    let kernel = ReplicatedCellKernel::new(ClusterNodeId::new(1), transport);

    let handle = kernel.create(CellKind::Exec).await;
    kernel.send(&handle, Message::new(Vec::new())).await.unwrap();

    let msg = kernel.receive(&handle).await.unwrap();
    assert!(msg.is_some());
}

#[tokio::test]
async fn replicated_cell_kernel_remote_send_routes_via_transport() {
    let transport = Arc::new(InMemoryClusterTransport::new());
    let kernel = ReplicatedCellKernel::new(ClusterNodeId::new(1), transport.clone());

    let remote_cell_id = axiom_kernel::CellId::new("remote-cell".to_string());
    kernel.place_cell(remote_cell_id.clone(), ClusterNodeId::new(2)).await;

    let handle = CellHandle { id: remote_cell_id, kind: CellKind::Exec };
    kernel.send(&handle, Message::new(Vec::new())).await.unwrap();

    let inbox = transport.drain_inbox(ClusterNodeId::new(2)).await;
    assert_eq!(inbox.len(), 1);
}

#[tokio::test]
async fn replicated_cell_kernel_receive_on_remote_cell_returns_error() {
    let transport = Arc::new(InMemoryClusterTransport::new());
    let kernel = ReplicatedCellKernel::new(ClusterNodeId::new(1), transport);

    let remote_cell_id = axiom_kernel::CellId::new("remote-cell".to_string());
    kernel.place_cell(remote_cell_id.clone(), ClusterNodeId::new(2)).await;

    let handle = CellHandle { id: remote_cell_id, kind: CellKind::Exec };
    let result = kernel.receive(&handle).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cluster_topology_add_peer_and_owner() {
    let mut topology = ClusterTopology::new(ClusterNodeId::new(1));
    topology.add_peer(ClusterNodeId::new(2), NodeAddress::new("host", 8080, "grpc"));

    let cell_id = axiom_kernel::CellId::new("cell-on-2".to_string());
    topology.place_cell(cell_id.clone(), ClusterNodeId::new(2));

    assert!(!topology.is_local(&cell_id));
    assert_eq!(topology.owner_of(&cell_id), Some(ClusterNodeId::new(2)));
}

#[tokio::test]
async fn state_sync_protocol_vector_clock_merge() {
    let local = axiom_kernel::signal::VectorClock::new();
    let mut remote = axiom_kernel::signal::VectorClock::new();
    remote.increment("cell-1");

    let merged = StateSyncProtocol::default_clock_merge(&local, &remote);
    assert_eq!(merged.get("cell-1"), 1);
}

#[tokio::test]
async fn state_sync_protocol_last_writer_wins_conflict_resolution() {
    let protocol = StateSyncProtocol::new(ConflictResolution::LastWriterWins);

    let local = SyncSnapshot {
        cell_id: axiom_kernel::CellId::new("c1".to_string()),
        owner_node: ClusterNodeId::new(1),
        state: serde_json::json!({"value": "local"}),
        vector_clock: axiom_kernel::signal::VectorClock::new(),
        updated_at_ns: 1000,
        schema_version: axiom_kernel::version::SchemaVersion::new(1),
    };

    let mut remote = local.clone();
    remote.state = serde_json::json!({"value": "remote"});
    remote.updated_at_ns = 2000;

    let resolved = protocol.resolve_conflict(&local, &remote);
    assert_eq!(resolved.state, serde_json::json!({"value": "remote"}));
}

#[tokio::test]
async fn cell_kernel_sync_state_produces_snapshot() {
    let kernel = axiom_kernel::CellKernel::new();
    let handle = kernel.create(CellKind::Exec).await;

    let snapshot = kernel
        .sync_state(&handle, ClusterNodeId::new(1))
        .await
        .unwrap();

    assert_eq!(snapshot.cell_id, handle.id);
    assert_eq!(snapshot.owner_node, ClusterNodeId::new(1));
}

