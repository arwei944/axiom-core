use crate::runtime::{RegisteredCell, RuntimeHealth};
use axiom_kernel::heatmap::collector::UsageSnapshot;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataSourceError {
    #[error("data source not initialized")]
    NotInitialized,
    #[error("failed to read cells")]
    CellReadFailed,
    #[error("failed to read health")]
    HealthReadFailed,
    #[error("failed to get heatmap")]
    HeatmapFailed,
    #[error("failed to subscribe to signals")]
    SignalSubscribeFailed,
}

pub trait RuntimeDataSource: Send + Sync {
    fn get_health(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RuntimeHealth, DataSourceError>> + Send + '_>,
    >;
    fn get_cells(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<RegisteredCell>, DataSourceError>>
                + Send
                + '_,
        >,
    >;
    fn get_entropy_snapshot(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<EntropySnapshotData, DataSourceError>>
                + Send
                + '_,
        >,
    >;
    fn get_heatmap(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<UsageSnapshot, DataSourceError>> + Send + '_>,
    >;
    fn subscribe_signals(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<SignalEventData>, DataSourceError>;
}

#[derive(Debug, Clone)]
pub struct EntropySnapshotData {
    pub global_value: f64,
    pub level: String,
    pub per_cell: Vec<(String, f64)>,
    pub last_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SignalEventData {
    pub ts_ns: u64,
    pub cell_id: String,
    pub signal_type: String,
    pub status: String,
}
