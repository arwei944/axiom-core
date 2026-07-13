use axiom_distributed::{ClusterConfig, ClusterView, NodeId, NodeInfo, NodeState};

fn main() {
    let node_a = NodeId::new("node-a");
    let node_b = NodeId::new("node-b");

    let info_a =
        NodeInfo::new("localhost:50051", vec![("zone".to_string(), "us-east".to_string())]);
    let info_b =
        NodeInfo::new("localhost:50052", vec![("zone".to_string(), "us-west".to_string())]);

    let mut cluster = ClusterView::default();
    cluster.nodes.push((node_a.clone(), info_a, NodeState::Alive));
    cluster.nodes.push((node_b.clone(), info_b, NodeState::Alive));

    let config = ClusterConfig::default();

    println!("=== distributed-cluster example ===");
    println!("local node: {:?}", config.node_id);
    println!("cluster size: {}", cluster.len());
    println!("nodes:");
    for (id, info, state) in &cluster.nodes {
        println!("  - {}: {} ({:?})", id, info.address, state);
    }
    println!(
        "In a real cluster, signals would route between nodes via the distributed kernel adapter."
    );
    println!("distributed-cluster example completed");
}
