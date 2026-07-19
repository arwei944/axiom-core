//! AxiomRuntime host for Agent Handoff path.

use crate::agent_cell::{
    new_shared_agent_outcome, AgentCell, AgentRunOutcome, SharedAgentOutcome, AGENT_CELL_ID,
    SIGNAL_HANDOFF,
};
use crate::run_log::{new_run_log, SharedRunLog};
use crate::surface::{build_surface_body, GovernorSnapshot};
use axiom_isa::{Governor, GovernorConfig, HandoffRequest};
use axiom_kernel::cell::RuntimeCellHandle;
use axiom_kernel::id::CellId;
use axiom_kernel::layer::RuntimeTier;
use axiom_runtime::{AxiomRuntime, CellRegistration, RuntimeBuilder, RuntimeConfig, RuntimeHealth};
use std::sync::Arc;
use std::time::Duration;

pub struct AgentHost {
    pub runtime: Arc<AxiomRuntime>,
    pub last: SharedAgentOutcome,
    pub runs: SharedRunLog,
    pub governor_snap: GovernorSnapshot,
}

pub struct HandoffRequestSpec {
    pub handoff: HandoffRequest,
    pub governor: GovernorConfig,
    pub trip_governor: bool,
    pub preload_entropy: bool,
}

impl Default for HandoffRequestSpec {
    fn default() -> Self {
        Self {
            handoff: HandoffRequest::new(
                "tok-1",
                "planner-agent",
                "executor-agent",
                "task_plan",
                "plan next deploy step",
            ),
            governor: GovernorConfig::default(),
            trip_governor: false,
            preload_entropy: false,
        }
    }
}

impl AgentHost {
    pub async fn boot(spec: &HandoffRequestSpec) -> Result<Self, String> {
        let last = new_shared_agent_outcome();
        let runs = new_run_log();
        let mut cell = AgentCell::with_config(spec.governor.clone(), last.clone(), runs.clone());
        if spec.trip_governor {
            cell.trip_governor();
        }
        if spec.preload_entropy {
            cell.preload_entropy();
        }

        // Snapshot governor state for surface before move into cell — rebuild metrics
        let gtmp = {
            let mut g = Governor::with_config(spec.governor.clone());
            if spec.trip_governor {
                g.trip();
            }
            if spec.preload_entropy {
                g.record_circuit_break();
            }
            GovernorSnapshot::from_governor(&g)
        };

        let handle = RuntimeCellHandle::new(Box::new(cell));
        let mut config = RuntimeConfig::default();
        config.dispatch_poll_interval_ms = 5;
        let runtime = Arc::new(
            RuntimeBuilder::new()
                .with_config(config)
                .auto_register_builtins(false)
                .build(),
        );

        runtime
            .register_cell(
                CellRegistration::new(CellId::new(AGENT_CELL_ID), RuntimeTier::Agent)
                    .with_cell(handle),
            )
            .await
            .map_err(|e| format!("register agent cell: {e}"))?;

        runtime
            .start()
            .await
            .map_err(|e| format!("start: {e}"))?;

        Ok(Self {
            runtime,
            last,
            runs,
            governor_snap: gtmp,
        })
    }

    pub async fn submit_handoff(&self, req: &HandoffRequest) -> Result<(), String> {
        let payload = serde_json::to_value(req).map_err(|e| e.to_string())?;
        self.runtime
            .publish_command(
                SIGNAL_HANDOFF,
                payload,
                Some(AGENT_CELL_ID),
                RuntimeTier::Agent,
            )
            .await
            .map_err(|e| format!("publish handoff: {e}"))?;
        Ok(())
    }

    pub async fn wait_outcome(&self, timeout: Duration) -> Result<AgentRunOutcome, String> {
        let start = std::time::Instant::now();
        loop {
            if let Ok(g) = self.last.lock() {
                if let Some(ref o) = *g {
                    return Ok(o.clone());
                }
            }
            if start.elapsed() > timeout {
                return Err("timeout waiting for AgentCell".into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn health(&self) -> RuntimeHealth {
        self.runtime.health().await
    }

    pub async fn surface_json(&self) -> serde_json::Value {
        let h = self.health().await;
        // Update snap score from last outcome if present
        let mut gov = self.governor_snap.clone();
        if let Ok(g) = self.last.lock() {
            if let Some(ref o) = *g {
                gov.level = o.governor_level.clone();
                gov.score = o.governor_score;
                gov.decision = if o.ok {
                    "allow".into()
                } else {
                    format!("reject:{}", o.error.clone().unwrap_or_default())
                };
            }
        }
        build_surface_body(&h, &gov, &[AGENT_CELL_ID], &self.runs)
    }

    pub async fn stop(self) {
        self.runtime.stop().await;
    }
}

pub async fn run_handoff(spec: HandoffRequestSpec) -> Result<AgentRunOutcome, String> {
    let host = AgentHost::boot(&spec).await?;
    host.submit_handoff(&spec.handoff).await?;
    let o = host.wait_outcome(Duration::from_secs(5)).await?;
    host.stop().await;
    Ok(o)
}
