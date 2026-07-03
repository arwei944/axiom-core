#[cfg(test)]
mod tests {
    use axiom_viz::{
        CellFlowRecord, CellFlowSnapshot, EntropyData, Timeline, TimelineEntry, TopologyGraph,
        VizSnapshot,
    };

    #[test]
    fn test_topology_graph_serializes() {
        let graph = TopologyGraph {
            cells: vec![],
            edges: vec![],
        };
        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.contains("cells"));
    }

    #[test]
    fn test_entropy_data_serializes() {
        let data = EntropyData {
            system_entropy: 1.0,
            cell_entropies: vec![("c1".into(), 2.0)],
            status: "Green".into(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("system_entropy"));
    }

    #[test]
    fn test_viz_snapshot_serializes() {
        let snapshot = VizSnapshot {
            topology: TopologyGraph {
                cells: vec![],
                edges: vec![],
            },
            timeline: Timeline { entries: vec![] },
            entropy: EntropyData {
                system_entropy: 0.0,
                cell_entropies: vec![],
                status: "Green".into(),
            },
            flow: CellFlowSnapshot { records: vec![] },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("topology"));
        assert!(json.contains("timeline"));
        assert!(json.contains("entropy"));
        assert!(json.contains("flow"));
    }

    #[test]
    fn test_cell_flow_record_serializes() {
        let record = CellFlowRecord {
            cell_id: "c1".into(),
            message_id: "m1".into(),
            kind: "signal".into(),
            from_layer: "Exec".into(),
            to_layer: "Agent".into(),
            timestamp_ns: 1,
            status: "ok".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("cell_id"));
    }
}
