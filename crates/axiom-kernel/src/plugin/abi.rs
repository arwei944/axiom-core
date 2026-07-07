pub type PluginResult<T> = Result<T, PluginError>;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("failed to load plugin: {0}")]
    LoadFailed(String),
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("missing symbol: {0}")]
    MissingSymbol(String),
    #[error("abi mismatch: expected {expected}, got {got}")]
    AbiMismatch { expected: String, got: String },
    #[error("initialization failed: {0}")]
    InitFailed(String),
    #[error("message handling failed: {0}")]
    HandleFailed(String),
    #[error("dependency missing: {0}")]
    DependencyMissing(String),
    #[error("dependency cycle detected: {0}")]
    DependencyCycle(String),
    #[error("unsupported plugin kind: {0}")]
    UnsupportedKind(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum PluginKind {
    #[default]
    Llm,
    Memory,
    Tool,
    Mcp,
    Planner,
    Alert,
    Viz,
    Governance,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilityDescriptor {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginStatus {
    Loaded,
    Initialized,
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginMessage {
    CallTool {
        tool: String,
        input: Vec<u8>,
    },
    QueryMemory {
        key: String,
    },
    SendSignal {
        signal: String,
        payload: Vec<u8>,
    },
    CheckAxiom {
        axiom: String,
        state: Vec<u8>,
    },
    QueryLens {
        lens: String,
        state: Vec<u8>,
    },
    Custom {
        kind: String,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PluginReply {
    Ok(Vec<u8>),
    Err(String),
}

pub trait AxiomPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn dependencies(&self) -> &[&'static str];
    fn capabilities(&self) -> &[CapabilityDescriptor];
    fn init(&mut self, ctx: PluginContext) -> PluginResult<()>;
    fn start(&mut self) -> PluginResult<()> {
        Ok(())
    }
    fn stop(&mut self) -> PluginResult<()> {
        Ok(())
    }
    fn handle_message(&mut self, msg: PluginMessage) -> PluginResult<PluginReply>;
    fn clone_box(&self) -> Box<dyn AxiomPlugin>;
}

pub struct PluginContext {
    pub cells: crate::CellKernel,
    pub signals: crate::SignalKernel,
    pub lens: crate::LensKernel,
    pub axioms: crate::AxiomKernel,
    pub witness: crate::WitnessKernel,
    pub plugins: crate::PluginRegistry,
    pub heatmap: std::sync::Arc<tokio::sync::RwLock<crate::HeatmapCollector>>,
    pub logger: PluginLogger,
    pub metrics: PluginMetrics,
}

impl PluginContext {
    pub fn new(
        cells: crate::CellKernel,
        signals: crate::SignalKernel,
        lens: crate::LensKernel,
        axioms: crate::AxiomKernel,
        witness: crate::WitnessKernel,
        plugins: crate::PluginRegistry,
        heatmap: std::sync::Arc<tokio::sync::RwLock<crate::HeatmapCollector>>,
    ) -> Self {
        Self {
            cells,
            signals,
            lens,
            axioms,
            witness,
            plugins,
            heatmap,
            logger: PluginLogger::new("plugin"),
            metrics: PluginMetrics::new(),
        }
    }

    pub async fn send_to_plugin(
        &self,
        _from: &str,
        to: &str,
        msg: PluginMessage,
    ) -> PluginResult<PluginReply> {
        let mut target = self.plugins.get(to).await.ok_or_else(|| {
            PluginError::LoadFailed(format!("plugin `{to}` not found"))
        })?;
        self.heatmap
            .write()
            .await
            .record_tool_invoke(to.to_string());
        Ok(target.handle_message(msg)?)
    }
}

#[derive(Clone)]
pub struct PluginLogger {
    pub target: &'static str,
}

impl PluginLogger {
    pub fn new(target: &'static str) -> Self {
        Self { target }
    }

    pub fn info(&self, msg: impl std::fmt::Display) {
        tracing::info!(target = self.target, "{}", msg);
    }

    pub fn warn(&self, msg: impl std::fmt::Display) {
        tracing::warn!(target = self.target, "{}", msg);
    }

    pub fn error(&self, msg: impl std::fmt::Display) {
        tracing::error!(target = self.target, "{}", msg);
    }
}

#[derive(Clone, Default)]
pub struct PluginMetrics {
    pub request_count: std::sync::Arc<tokio::sync::RwLock<u64>>,
}

impl PluginMetrics {
    pub fn new() -> Self {
        Self {
            request_count: std::sync::Arc::new(tokio::sync::RwLock::new(0)),
        }
    }

    pub async fn record_request(&self) {
        let mut count = self.request_count.write().await;
        *count += 1;
    }

    pub async fn request_count(&self) -> u64 {
        *self.request_count.read().await
    }
}
