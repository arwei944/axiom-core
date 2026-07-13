//! Axiom Core 快速开始示例
//!
//! 本示例演示如何：
//! 1. 初始化 Axiom 运行时
//! 2. 注册一个简单的 Echo Cell
//! 3. 发送信号并打印结果
//! 4. 启动 API 服务器

use axiom_api::ApiServerBuilder;
use axiom_kernel::cell::SupervisionStrategy;
use axiom_kernel::clock::global_clock;
use axiom_kernel::id::{CellId, CorrelationId, MsgId};
use axiom_kernel::layer::RuntimeTier;
use axiom_kernel::signal::{SignalKind, VectorClock};
use axiom_kernel::SchemaVersion;
use axiom_oversight::{
    ComplianceGuardCell, EntropyGovernorCell, HealthCollectorCell, OversightKernelAdapter,
};
use axiom_runtime::{AxiomRuntime, CellRegistration, RuntimeConfig};
use std::net::SocketAddr;
use std::sync::Arc;

const ECHO_CELL_ID: &str = "echo-cell";
const SIGNAL_TYPE: &str = "EchoCommand";
const API_ADDR: &str = "0.0.0.0:9092";

fn make_signal_envelope(target_cell: &str) -> axiom_kernel::signal::SignalEnvelope {
    axiom_kernel::signal::SignalEnvelope {
        msg_id: MsgId::new("echo-cmd"),
        correlation_id: CorrelationId::new("echo-corr"),
        trace_id: None,
        signal_type: SIGNAL_TYPE.to_string(),
        vector_clock: VectorClock::new(),
        timestamp_ns: global_clock().now_ns(),
        kind: SignalKind::Command,
        source_layer: RuntimeTier::Oversight,
        target_layer: RuntimeTier::Exec,
        source_cell: None,
        target_cell: Some(target_cell.to_string()),
        payload: serde_json::json!({"message": "Hello, Axiom!"}),
        schema_version: SchemaVersion::new(1),
        parent_msg_id: None,
        hop_count: 0,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("=== Axiom Core 快速开始示例 ===");

    // 1. 初始化运行时
    let config = RuntimeConfig::default();
    let runtime = Arc::new(AxiomRuntime::new(config));

    tracing::info!("运行时已创建");

    // 2. 注册一个简单的 Echo Cell
    let registration = CellRegistration {
        id: CellId::new(ECHO_CELL_ID),
        layer: RuntimeTier::Exec,
        version: axiom_kernel::version::Version::new(0, 1, 0),
        supervision_strategy: SupervisionStrategy::Restart { max_retries: 3 },
        cell: None,
        factory: None,
    };

    runtime.register_cell(registration).await?;
    tracing::info!("Echo Cell 已注册: {}", ECHO_CELL_ID);

    // 3. 启动运行时
    runtime.start().await?;
    tracing::info!("运行时已启动");

    // 4. 发送信号并打印结果
    let envelope = make_signal_envelope(ECHO_CELL_ID);
    tracing::info!(
        "发送信号: type={}, target={}",
        envelope.signal_type,
        envelope.target_cell.as_deref().unwrap_or("unknown")
    );

    let delivered = runtime.bus().publish(envelope).await?;
    tracing::info!("信号投递成功，投递到 {} 个 Cell", delivered);

    // 等待信号处理
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. 创建监督适配器
    let oversight = Arc::new(OversightKernelAdapter::new(
        Arc::new(EntropyGovernorCell::default()),
        Arc::new(HealthCollectorCell::new()),
        Arc::new(ComplianceGuardCell::new()),
    ));

    // 6. 启动 API 服务器
    let addr: SocketAddr = API_ADDR.parse()?;
    let server = ApiServerBuilder::new().addr(addr).development().build(runtime.clone(), oversight);

    tracing::info!("API 服务器启动在 http://{}", addr);
    tracing::info!("可用的 API 端点:");
    tracing::info!("  GET /api/v1/health   - 健康检查");
    tracing::info!("  GET /api/v1/cells    - Cell 列表");
    tracing::info!("  GET /api/v1/entropy  - 熵值状态");
    tracing::info!("  GET /api/v1/heatmap  - 活动热图");
    tracing::info!("  GET /api/v1/metrics   - Prometheus 指标");
    tracing::info!("按 Ctrl+C 停止服务...");

    // 运行 API 服务器，等待 Ctrl+C 信号
    tokio::select! {
        result = server.serve() => {
            if let Err(e) = result {
                tracing::error!("API 服务器错误: {}", e);
            }
            tracing::info!("API 服务器已停止");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("收到 Ctrl+C 信号，正在停止...");
        }
    }

    // 停止运行时
    runtime.stop().await;
    tracing::info!("运行时已停止");
    tracing::info!("=== 示例结束 ===");

    Ok(())
}
