use crate::types::{ApiCell, ApiEntropy, ApiHealth, ApiHeatmap};
use axiom_runtime::RuntimeDataSource;
use axiom_oversight::OversightDataSource;
use axiom_kernel::layer::RuntimeTier;
use std::sync::Arc;

pub struct HealthAggregator {
    runtime_data: Arc<dyn RuntimeDataSource>,
    #[allow(dead_code)]
    oversight_data: Arc<dyn OversightDataSource>,
}

impl HealthAggregator {
    pub fn new(
        runtime_data: Arc<dyn RuntimeDataSource>,
        oversight_data: Arc<dyn OversightDataSource>,
    ) -> Self {
        Self {
            runtime_data,
            oversight_data,
        }
    }

    pub async fn aggregate(&self) -> Result<ApiHealth, crate::types::ApiError> {
        let runtime_health = self.runtime_data.get_health().await?;
        let entropy_snapshot = self.runtime_data.get_entropy_snapshot().await?;

        let status = if runtime_health.started {
            "ok".to_string()
        } else {
            "stopped".to_string()
        };

        Ok(ApiHealth {
            status,
            cells_running: runtime_health.cells_running,
            cells_stopped: runtime_health.cells_stopped,
            total_restarts: runtime_health.total_restarts,
            messages_delivered: runtime_health.messages_delivered,
            messages_rejected: runtime_health.messages_rejected,
            entropy_score: entropy_snapshot.global_value,
            entropy_level: entropy_snapshot.level,
            preflight_passed: runtime_health.preflight_passed,
            uptime_ms: runtime_health.uptime_ms,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

pub struct CellAggregator {
    runtime_data: Arc<dyn RuntimeDataSource>,
}

impl CellAggregator {
    pub fn new(runtime_data: Arc<dyn RuntimeDataSource>) -> Self {
        Self { runtime_data }
    }

    pub async fn get_cells(&self) -> Result<Vec<ApiCell>, crate::types::ApiError> {
        let cells = self.runtime_data.get_cells().await?;
        Ok(cells
            .into_iter()
            .map(|c| ApiCell {
                id: c.id().to_string(),
                layer: match c.layer() {
                    RuntimeTier::Exec => "Exec".to_string(),
                    RuntimeTier::Oversight => "Oversight".to_string(),
                    RuntimeTier::Agent => "Agent".to_string(),
                    RuntimeTier::Validate => "Validate".to_string(),
                },
                state: "Running".to_string(),
                version: c.version().to_string(),
            })
            .collect())
    }
}

pub struct HeatmapAggregator {
    runtime_data: Arc<dyn RuntimeDataSource>,
}

impl HeatmapAggregator {
    pub fn new(runtime_data: Arc<dyn RuntimeDataSource>) -> Self {
        Self { runtime_data }
    }

    pub async fn get_heatmap(&self) -> Result<ApiHeatmap, crate::types::ApiError> {
        let snapshot = self.runtime_data.get_heatmap().await?;
        Ok(ApiHeatmap {
            timestamp: snapshot.timestamp,
            hot_cells: snapshot.hot_cells,
            hot_signals: snapshot.hot_signals,
            hot_tools: snapshot.hot_tools,
        })
    }
}

pub struct EntropyAggregator {
    runtime_data: Arc<dyn RuntimeDataSource>,
}

impl EntropyAggregator {
    pub fn new(runtime_data: Arc<dyn RuntimeDataSource>) -> Self {
        Self { runtime_data }
    }

    pub async fn get_entropy(&self) -> Result<ApiEntropy, crate::types::ApiError> {
        let snapshot = self.runtime_data.get_entropy_snapshot().await?;
        Ok(ApiEntropy {
            value: snapshot.global_value,
            level: snapshot.level,
            per_cell: snapshot.per_cell,
            last_action: snapshot.last_action,
        })
    }
}