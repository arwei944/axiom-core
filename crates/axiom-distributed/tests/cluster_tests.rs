#[cfg(test)]
mod tests {
    use axiom_distributed::{
        cluster::ClusterView,
        discovery::NodeDiscovery,
        node::{NodeId, NodeInfo, NodeState},
        sync::{EventSync, SyncRequest},
        witness::WitnessSync,
    };

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new("node-1");
        assert!(id.to_string().starts_with("node-"));
    }

    #[test]
    fn test_cluster_view_contains() {
        let info = NodeInfo::new("127.0.0.1:8080", Vec::new());
        let node_id = info.node_id;
        let mut view = ClusterView::default();
        view.nodes.push((node_id, info, NodeState::Alive));
        assert!(view.contains(node_id));
    }

    #[tokio::test]
    async fn test_discovery_subscribe() {
        let local = NodeInfo::new("127.0.0.1:8081", Vec::new());
        let disc = NodeDiscovery::new(Default::default(), local);
        let mut rx = disc.subscribe();
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_event_sync_state_advances() {
        let node = NodeId::new("sync-node");
        let sync = EventSync::new(node);
        let req = SyncRequest {
            source: node,
            target: node,
            from_sequence: 0,
            to_sequence: 10,
        };
        let resp = sync.sync(req).await.unwrap();
        sync.apply(resp).await.unwrap();
    }

    #[test]
    fn test_witness_sync_validation_passes() {
        let ws = WitnessSync::new(NodeId::new("ws"));
        assert!(ws.validate_chain(&[]).is_ok());
    }
}
