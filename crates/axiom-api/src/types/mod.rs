use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "error")]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("internal server error: {0}")]
    InternalError(String),
    #[error("bad request: {0}")]
    BadRequest(String),
}

impl From<axiom_runtime::DataSourceError> for ApiError {
    fn from(e: axiom_runtime::DataSourceError) -> Self {
        match e {
            axiom_runtime::DataSourceError::NotInitialized => {
                Self::InternalError("data source not initialized".to_string())
            }
            axiom_runtime::DataSourceError::CellReadFailed => {
                Self::InternalError("failed to read cells".to_string())
            }
            axiom_runtime::DataSourceError::HealthReadFailed => {
                Self::InternalError("failed to read health".to_string())
            }
            axiom_runtime::DataSourceError::HeatmapFailed => {
                Self::InternalError("failed to get heatmap".to_string())
            }
            axiom_runtime::DataSourceError::SignalSubscribeFailed => {
                Self::InternalError("failed to subscribe to signals".to_string())
            }
        }
    }
}

impl From<axiom_oversight::OversightDataSourceError> for ApiError {
    fn from(e: axiom_oversight::OversightDataSourceError) -> Self {
        match e {
            axiom_oversight::OversightDataSourceError::NotInitialized => {
                Self::InternalError("oversight data source not initialized".to_string())
            }
            axiom_oversight::OversightDataSourceError::HealthFailed => {
                Self::InternalError("failed to get system health".to_string())
            }
            axiom_oversight::OversightDataSourceError::EntropyFailed => {
                Self::InternalError("failed to get entropy status".to_string())
            }
            axiom_oversight::OversightDataSourceError::ComplianceFailed => {
                Self::InternalError("failed to get compliance report".to_string())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiHealth {
    pub status: String,
    pub cells_running: u64,
    pub cells_stopped: u64,
    pub total_restarts: u64,
    pub messages_delivered: u64,
    pub messages_rejected: u64,
    pub entropy_score: f64,
    pub entropy_level: String,
    pub preflight_passed: bool,
    pub uptime_ms: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiCell {
    pub id: String,
    pub layer: String,
    pub state: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiHeatmap {
    pub timestamp: u64,
    pub hot_cells: Vec<(String, u64)>,
    pub hot_signals: Vec<(String, u64)>,
    pub hot_tools: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiEntropy {
    pub value: f64,
    pub level: String,
    pub per_cell: Vec<(String, f64)>,
    pub last_action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiSignalEvent {
    pub ts_ns: u64,
    pub cell_id: String,
    pub signal_type: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_health_serialization() {
        let health = ApiHealth {
            status: "ok".to_string(),
            cells_running: 5,
            cells_stopped: 0,
            total_restarts: 0,
            messages_delivered: 100,
            messages_rejected: 5,
            entropy_score: 0.5,
            entropy_level: "green".to_string(),
            preflight_passed: true,
            uptime_ms: 10000,
            version: "0.4.0".to_string(),
        };
        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"cells_running\":5"));
    }

    #[test]
    fn api_cell_serialization() {
        let cell = ApiCell {
            id: "test-cell".to_string(),
            layer: "Exec".to_string(),
            state: "Running".to_string(),
            version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&cell).unwrap();
        assert!(json.contains("\"id\":\"test-cell\""));
        assert!(json.contains("\"layer\":\"Exec\""));
    }
}
