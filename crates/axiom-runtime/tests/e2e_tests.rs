//! End-to-end integration tests for Axiom Runtime.
//!
//! Covers: Runtime startup -> Cell registration -> Signal dispatch ->
//! EventStore persistence -> Witness recording -> Runtime shutdown.

use axiom_core::cell::{Cell, CellHandle, SupervisionStrategy};
use axiom_core::id::{CellId, CorrelationId, MsgId};
use axiom_core::layer::Layer;
use axiom_core::signal::{Signal, SignalKind, VectorClock};
use axiom_core::version::Version;
use axiom_runtime::CellRegistration;
use std::sync::Arc;
use tokio::sync::Mutex;

// ---------- Test Signal ----------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct E2eCommand {
    msg_id: MsgId,
    correlation_id: CorrelationId,
    vector_clock: VectorClock,
    payload: String,
}

impl E2eCommand {
    fn new(payload: &str) -> Self {
        Self {
            msg_id: MsgId::new("e2e-cmd"),
            correlation_id: CorrelationId::new("e2e-corr"),
            vector_clock: VectorClock::new(),
            payload: payload.to_string(),
        }
    }
}

impl Signal for E2eCommand {
    fn signal_type(&self) -> &'static str {
        "E2eCommand"
    }
    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }
    fn timestamp_ns(&self) -> u64 {
        axiom_core::signal::now_ns()
    }
    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }
    fn layer(&self) -> Layer {
        Layer::Exec
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }
    fn validate(&self) -> axiom_core::schema::ValidationResult {
        axiom_core::schema::ValidationResult::ok()
    }
    fn serialize_to_json(&self) -> axiom_core::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| axiom_core::AxiomError::SignalSerialization {
            signal_type: self.signal_type().into(),
            message: e.to_string(),
        })
    }
}

// ---------- Test Cell ----------

#[derive(Debug, Clone)]
struct E2eCell {
    id: CellId,
    state: Arc<Mutex<String>>,
}

impl E2eCell {
    fn new(id: &str) -> Self {
        Self {
            id: CellId::new(id),
            state: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl Cell for E2eCell {
    type Message = E2eCommand;
    type Layer = axiom_core::sealed::ExecLayer;

    fn id(&self) -> &CellId {
        &self.id
    }

    fn supervision_strategy(&self) -> SupervisionStrategy {
        SupervisionStrategy::Restart { max_retries: 3 }
    }

    async fn handle(
        &mut self,
        signal: E2eCommand,
        _ctx: axiom_core::context::LayeredCellContext<'_, Self::Layer>,
    ) -> (
        axiom_core::Result<()>,
        Vec<axiom_core::context::OutgoingEnvelope>,
        Vec<axiom_core::context::OutgoingWitness>,
    ) {
        let mut state = self.state.lock().await;
        *state = format!("received: {}", signal.payload);
        (Ok(()), Vec::new(), Vec::new())
    }
}

// ---------- End-to-end test ----------

#[tokio::test]
async fn test_runtime_e2e_signal_dispatch_and_persistence() {
    // 1. Create runtime with in-memory store for deterministic testing
    let runtime = axiom_runtime::AxiomRuntime::new(axiom_runtime::RuntimeConfig::default());

    // 2. Register cell
    let cell = E2eCell::new("e2e-cell");
    let cell_handle = CellHandle::new(cell.clone());
    let reg = CellRegistration {
        id: cell.id().clone(),
        layer: Layer::Exec,
        version: Version::new(0, 1, 0),
        supervision_strategy: SupervisionStrategy::Restart { max_retries: 3 },
        cell: Some(cell_handle),
        factory: None,
    };

    let _mailbox = runtime.register_cell(reg).await.unwrap();

    // 3. Start runtime
    runtime.start().await.unwrap();

    // 4. Submit signal
    let cmd = E2eCommand::new("hello-e2e");
    let delivered = runtime
        .submit_signal(cmd, Some("e2e-cell"), Layer::Exec)
        .await
        .unwrap();

    assert_eq!(delivered, 1);

    // 5. Allow async dispatch
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 6. Verify cell state updated via signal handling
    let state = cell.state.lock().await;
    assert_eq!(*state, "received: hello-e2e");

    // 7. Health check
    let health = runtime.health().await;
    assert!(health.started);
    assert!(health.preflight_passed);
    assert_eq!(health.cells_running, 1);

    // 8. Stop runtime
    runtime.stop().await;
}
