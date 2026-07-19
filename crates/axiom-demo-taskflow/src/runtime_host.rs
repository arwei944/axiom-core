//! Commercial host: real AxiomRuntime + TaskCell registration.

use crate::pipeline::FailMode;
use crate::task_cell::{
    new_shared_outcome, SharedOutcome, TaskCell, TaskRunOutcome, SIGNAL_SUBMIT, TASK_CELL_ID,
};
use axiom_isa::GovernorConfig;
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::id::CellId;
use axiom_kernel::layer::RuntimeTier;
use axiom_runtime::{AxiomRuntime, CellRegistration, RuntimeBuilder, RuntimeConfig, RuntimeHealth};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub struct RuntimeHost {
    pub runtime: Arc<AxiomRuntime>,
    pub last: SharedOutcome,
}

pub struct RunRequest {
    pub fail: FailMode,
    pub governor: GovernorConfig,
    pub payload: Value,
    /// When true, trip governor before first signal (melt path).
    pub trip_governor: bool,
    /// Pre-record circuit_break entropy (melt-style without force_open).
    pub preload_entropy: bool,
    /// How many sequential SubmitTask signals to send (fail path uses >1).
    pub submissions: u32,
}

impl Default for RunRequest {
    fn default() -> Self {
        Self {
            fail: FailMode::None,
            governor: GovernorConfig::default(),
            payload: serde_json::json!({
                "title": "ship-mvp",
                "priority": 2,
                "payload": "wire four primitives"
            }),
            trip_governor: false,
            preload_entropy: false,
            submissions: 1,
        }
    }
}

impl RuntimeHost {
    pub async fn boot(req: &RunRequest) -> Result<Self, String> {
        let last = new_shared_outcome();
        let mut cell = TaskCell::with_governor_config(req.fail, req.governor.clone(), last.clone());
        if req.trip_governor {
            cell.trip_governor();
        }
        if req.preload_entropy {
            cell.preload_circuit_break_entropy();
        }

        let handle = RuntimeCellHandle::new(Box::new(cell));
        let mut config = RuntimeConfig::default();
        config.dispatch_poll_interval_ms = 5;
        let runtime = Arc::new(
            RuntimeBuilder::new()
                .with_config(config)
                // Fewer interceptors → deterministic commercial demo path
                .auto_register_builtins(false)
                .build(),
        );

        runtime
            .register_cell(
                CellRegistration::new(CellId::new(TASK_CELL_ID), RuntimeTier::Exec)
                    .with_cell(handle),
            )
            .await
            .map_err(|e| format!("register_cell: {e}"))?;

        // Override supervision after register — CellRegistration builder doesn't
        // expose a with_supervision helper in all versions; set via register fields.
        // Registration already used default Restart strategy which is fine.

        runtime
            .start()
            .await
            .map_err(|e| format!("runtime start: {e}"))?;

        Ok(Self { runtime, last })
    }

    pub async fn health(&self) -> RuntimeHealth {
        self.runtime.health().await
    }

    pub async fn submit_once(&self, payload: Value) -> Result<(), String> {
        self.runtime
            .publish_command(
                SIGNAL_SUBMIT,
                payload,
                Some(TASK_CELL_ID),
                RuntimeTier::Exec,
            )
            .await
            .map_err(|e| format!("publish: {e}"))?;
        Ok(())
    }

    pub async fn wait_outcome(&self, timeout: Duration) -> Result<TaskRunOutcome, String> {
        let start = std::time::Instant::now();
        loop {
            if let Ok(g) = self.last.lock() {
                if let Some(ref o) = *g {
                    return Ok(o.clone());
                }
            }
            if start.elapsed() > timeout {
                return Err("timeout waiting for TaskCell outcome".into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn clear_outcome(&self) {
        if let Ok(mut g) = self.last.lock() {
            *g = None;
        }
    }

    pub async fn stop(self) {
        self.runtime.stop().await;
    }
}

/// Run commercial path end-to-end; returns the last outcome (or first reject).
pub async fn run_commercial(req: RunRequest) -> Result<Vec<TaskRunOutcome>, String> {
    let host = RuntimeHost::boot(&req).await?;
    let mut outcomes = Vec::new();

    for i in 0..req.submissions.max(1) {
        host.clear_outcome().await;
        host.submit_once(req.payload.clone())
            .await
            .map_err(|e| format!("submit #{i}: {e}"))?;
        let o = host
            .wait_outcome(Duration::from_secs(5))
            .await
            .map_err(|e| format!("wait #{i}: {e}"))?;
        outcomes.push(o);
    }

    host.stop().await;
    Ok(outcomes)
}


