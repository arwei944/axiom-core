use axiom_kernel::cluster::{ClusterNodeId, ClusterTopology, InMemoryClusterTransport, NodeAddress};

fn main() {
    let node_a = ClusterNodeId::new(1);
    let node_b = ClusterNodeId::new(2);

    let mut topology = ClusterTopology::new(node_a.clone());
    topology.add_peer(node_b.clone(), NodeAddress::new("localhost", 50051, "http"));

    let _transport = InMemoryClusterTransport::new();

    println!("=== distributed-cluster example ===");
    println!("local node: {:?}", topology.local_node);
    println!("peers: {:?}", topology.peers.keys().cloned().collect::<Vec<_>>());
    println!("transport ready: true");
    println!("In a real cluster, signals would route between nodes.");
    println!("distributed-cluster example completed");
}
