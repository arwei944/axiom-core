//! Kernel integration for `axiom-viz`.
//!
//! Provides adapters so visualization snapshots can be produced from
//! kernel runtime state.

use crate::VizSnapshot;

/// Adapter that builds visualization snapshots from kernel state.
pub struct VizKernelAdapter {
    snapshots: Vec<VizSnapshot>,
}

impl Default for VizKernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl VizKernelAdapter {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn push_snapshot(&mut self, snapshot: VizSnapshot) {
        self.snapshots.push(snapshot);
    }

    pub fn snapshots(&self) -> &[VizSnapshot] {
        &self.snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::TopologyGraph;

    #[test]
    fn adapter_records_snapshots() {
        let mut adapter = VizKernelAdapter::new();
        adapter.push_snapshot(VizSnapshot {
            topology: TopologyGraph {
                cells: Vec::new(),
                edges: Vec::new(),
            },
            timeline: crate::Timeline {
                entries: Vec::new(),
            },
            entropy: crate::EntropyData {
                system_entropy: 0.0,
                cell_entropies: Vec::new(),
                status: "Green".into(),
            },
            flow: crate::CellFlowSnapshot {
                records: Vec::new(),
            },
        });
        assert_eq!(adapter.snapshots().len(), 1);
    }
}
