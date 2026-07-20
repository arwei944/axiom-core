//! ULE commercial product library (task + agent handoff + surface + lens + plugins).

pub mod agent_cell;
pub mod agent_host;
pub mod alert_bridge;
pub mod events;
pub mod health;
pub mod lenses;
pub mod llm_port;
pub mod metrics;
pub mod pipeline;
pub mod plugin_host;
pub mod product_gateway;
pub mod run_log;
pub mod runtime_host;
pub mod store;
pub mod surface;
pub mod task_cell;
pub mod workbench;

pub use agent_cell::{AgentRunOutcome, AGENT_CELL_ID, SIGNAL_HANDOFF};
pub use agent_host::{run_handoff, AgentHost, HandoffRequestSpec};
pub use events::{new_event_bus, DomainEvent, EventBus, SharedEventBus};
pub use lenses::{list_lens_ids, project_lens, LENS_GOVERNOR, LENS_HEALTH, LENS_METRICS, LENS_RUNS};
pub use metrics::{new_metrics, ProductMetrics, SharedMetrics};
pub use pipeline::{FailMode, TaskPipeline, TaskResult};
pub use plugin_host::{ProductPluginHost, PluginSurfaceInfo};
pub use product_gateway::{boot_write_runtime, GatewayConfig, ProductGateway, WriteRuntime};
pub use runtime_host::{run_commercial, RunRequest, RuntimeHost};
pub use task_cell::{TaskCell, TaskRunOutcome, SIGNAL_SUBMIT, TASK_CELL_ID};
pub use workbench::{run_workbench, workbench_composer, workbench_composer_with_limits};
