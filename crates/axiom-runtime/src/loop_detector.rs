//! Message loop detector using correlation path tracking.

use axiom_kernel::signal::SignalEnvelope;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet, VecDeque};

type Result<T> = std::result::Result<T, axiom_kernel::KernelError>;

struct CorrelationTrack {
    cells: HashSet<String>,
}

struct LruCorrelationMap {
    map: HashMap<String, CorrelationTrack>,
    order: VecDeque<String>,
    max_tracked: usize,
}

impl LruCorrelationMap {
    fn new(max_tracked: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_tracked,
        }
    }
    fn get_or_default(&mut self, key: &str) -> &mut HashSet<String> {
        let k = key.to_string();
        if !self.map.contains_key(&k) {
            self.evict_if_needed();
            self.order.push_back(k.clone());
            self.map.insert(
                k.clone(),
                CorrelationTrack {
                    cells: HashSet::new(),
                },
            );
        }
        &mut self // foxguard: ignore[rs/no-unwrap-in-lib] — key was just inserted in get_or_default
            .map
            .get_mut(&k)
            .expect("key just inserted should exist")
            .cells
    }
    fn evict_if_needed(&mut self) {
        while self.map.len() >= self.max_tracked {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
    }
    fn len(&self) -> usize {
        self.map.len()
    }
}

pub struct LoopDetector {
    paths: RwLock<LruCorrelationMap>,
    max_cells_per_correlation: usize,
}

impl LoopDetector {
    pub fn new(max_cells_per_correlation: usize, max_tracked: usize) -> Self {
        Self {
            paths: RwLock::new(LruCorrelationMap::new(max_tracked)),
            max_cells_per_correlation,
        }
    }

    pub fn check_and_record(&self, env: &SignalEnvelope) -> Result<()> {
        let cid = env.correlation_id.as_str().to_string();
        let target = env
            .target_cell
            .clone()
            .unwrap_or_else(|| format!("layer:{:?}", env.target_layer));

        let mut paths = self.paths.write();
        let cells = paths.get_or_default(&cid);

        if cells.contains(&target) && cells.len() >= 2 {
            return Err(axiom_kernel::KernelError::LoopDetected {
                message: format!(
                    "revisiting cell {} after visiting {} cells",
                    target,
                    cells.len()
                ),
                correlation_id: cid,
            });
        }

        if cells.len() >= self.max_cells_per_correlation {
            return Err(axiom_kernel::KernelError::LoopDetected {
                message: format!(
                    "visited {} cells (max {})",
                    cells.len(),
                    self.max_cells_per_correlation
                ),
                correlation_id: cid,
            });
        }

        cells.insert(target);
        Ok(())
    }

    pub fn tracked_count(&self) -> usize {
        self.paths.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_kernel::id::{CorrelationId, MsgId};
    use axiom_kernel::layer::Layer;
    use axiom_kernel::signal::{SignalKind, VectorClock};

    fn env(target: &str, cid: &str) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new(format!("m-{}-{}", cid, target)),
            correlation_id: CorrelationId::new(cid),
            trace_id: None,
            signal_type: "t".to_string(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: Layer::Exec,
            target_layer: Layer::Exec,
            source_cell: None,
            target_cell: Some(target.to_string()),
            payload: serde_json::Value::Null,
            schema_version: axiom_kernel::version::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn test_normal_chain_passes() {
        let d = LoopDetector::new(5, 100);
        assert!(d.check_and_record(&env("a", "c1")).is_ok());
        assert!(d.check_and_record(&env("b", "c1")).is_ok());
        assert!(d.check_and_record(&env("c", "c1")).is_ok());
    }

    #[test]
    fn test_loop_detected_on_revisit() {
        let d = LoopDetector::new(5, 100);
        assert!(d.check_and_record(&env("a", "c1")).is_ok());
        assert!(d.check_and_record(&env("b", "c1")).is_ok());
        assert!(d.check_and_record(&env("a", "c1")).is_err());
    }

    #[test]
    fn test_long_chain_rejected() {
        let d = LoopDetector::new(2, 100);
        assert!(d.check_and_record(&env("a", "c1")).is_ok());
        assert!(d.check_and_record(&env("b", "c1")).is_ok());
        assert!(d.check_and_record(&env("c", "c1")).is_err());
    }

    #[test]
    fn test_lru_eviction() {
        let d = LoopDetector::new(5, 3);
        d.check_and_record(&env("a", "c1")).unwrap();
        d.check_and_record(&env("a", "c2")).unwrap();
        d.check_and_record(&env("a", "c3")).unwrap();
        assert_eq!(d.tracked_count(), 3);
        d.check_and_record(&env("a", "c4")).unwrap();
        assert_eq!(d.tracked_count(), 3);
    }
}