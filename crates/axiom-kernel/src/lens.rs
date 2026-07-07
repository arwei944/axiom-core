use crate::axiom::{KernelError, KernelResult, Projection, State};
use crate::HeatmapCollector;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LensKernel {
    lenses: RwLock<HashMap<String, Arc<dyn crate::axiom::DynLens>>>,
    heatmap: std::sync::Arc<RwLock<HeatmapCollector>>,
}

impl LensKernel {
    pub fn new() -> Self {
        Self {
            lenses: RwLock::new(HashMap::new()),
            heatmap: std::sync::Arc::new(RwLock::new(HeatmapCollector::new())),
        }
    }

    pub fn with_heatmap(heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self {
            lenses: RwLock::new(HashMap::new()),
            heatmap,
        }
    }

    pub fn heatmap(&self) -> std::sync::Arc<RwLock<HeatmapCollector>> {
        self.heatmap.clone()
    }

    pub async fn register(&self, lens: Arc<dyn crate::axiom::DynLens>) {
        let mut lenses = self.lenses.write().await;
        lenses.insert(lens.id().to_string(), lens);
    }

    pub async fn query(&self, id: &str, state: &State) -> KernelResult<Projection> {
        let lenses = self.lenses.read().await;
        if let Some(lens) = lenses.get(id) {
            let result = lens.project(state);
            drop(lenses);
            if result.is_ok() {
                self.heatmap.write().await.record_lens_query(id.to_string());
            }
            result
        } else {
            Err(KernelError::LensNotFound {
                lens_id: id.to_string(),
            })
        }
    }
}

impl Default for LensKernel {
    fn default() -> Self {
        Self::new()
    }
}
