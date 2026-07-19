//! Task pipeline as a real runtime Cell (Composer-in-Cell under AxiomRuntime).

use crate::pipeline::{FailMode, TaskPipeline, TaskResult};
use crate::store::InMemoryTaskStore;
use axiom_isa::{Composer, Governor, GovernorConfig, IsaError, WitnessJournal};
use axiom_kernel::cell::{BoxHandleFuture, DynCell, DynHandleCell};
use axiom_kernel::context::{CellContext, OutgoingWitness};
use axiom_kernel::id::CellId;
use axiom_kernel::signal::SignalEnvelope;
use axiom_kernel::witness::Witness;
use axiom_kernel::RuntimeTier;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub const TASK_CELL_ID: &str = "task-cell";
pub const SIGNAL_SUBMIT: &str = "SubmitTask";

/// Outcome of one Signal handled by [`TaskCell`].
#[derive(Debug, Clone)]
pub struct TaskRunOutcome {
    pub ok: bool,
    pub error: Option<String>,
    pub result: Option<TaskResult>,
    pub witnesses: Vec<Witness>,
    pub governor_level: String,
    pub governor_score: f64,
    pub circuit: String,
}

/// Shared slot so the host can observe the last Cell handle result.
pub type SharedOutcome = Arc<Mutex<Option<TaskRunOutcome>>>;

pub fn new_shared_outcome() -> SharedOutcome {
    Arc::new(Mutex::new(None))
}

/// Exec-tier Cell: Governor admit → Composer → Witness (returned to runtime).
pub struct TaskCell {
    id: CellId,
    pipeline: TaskPipeline,
    governor: Governor,
    last: SharedOutcome,
}

impl TaskCell {
    pub fn new(fail: FailMode, governor: Governor, last: SharedOutcome) -> Self {
        Self {
            id: CellId::new(TASK_CELL_ID),
            pipeline: TaskPipeline::new(InMemoryTaskStore::new(), fail),
            governor,
            last,
        }
    }

    pub fn with_defaults(fail: FailMode) -> (Self, SharedOutcome) {
        let last = new_shared_outcome();
        let cell = Self::new(fail, Governor::for_demo(), last.clone());
        (cell, last)
    }

    pub fn with_governor_config(
        fail: FailMode,
        config: GovernorConfig,
        last: SharedOutcome,
    ) -> Self {
        Self::new(fail, Governor::with_config(config), last)
    }

    pub fn trip_governor(&mut self) {
        self.governor.trip();
    }

    pub fn preload_circuit_break_entropy(&mut self) {
        self.governor.record_circuit_break();
    }

    fn store_outcome(&self, outcome: TaskRunOutcome) {
        if let Ok(mut g) = self.last.lock() {
            *g = Some(outcome);
        }
    }

    fn handle_signal<'a>(
        &mut self,
        env: &SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> (bool, Option<TaskResult>, Option<String>, Vec<OutgoingWitness>) {
        // Runtime already called begin_processing; re-bind correlation if empty path.
        let mut journal = WitnessJournal::new(ctx);

        if env.signal_type != SIGNAL_SUBMIT {
            let msg = format!("unsupported signal_type {}", env.signal_type);
            let _ = journal.record_err(
                axiom_isa::StepKind::Composer,
                "task-cell",
                &msg,
            );
            let ws = wrap_witnesses(journal.into_witnesses());
            return (false, None, Some(msg), ws);
        }

        if let Err(e) = self.governor.admit(&mut journal) {
            self.governor.record_rejected();
            let ws = wrap_witnesses(journal.into_witnesses());
            return (false, None, Some(e.to_string()), ws);
        }

        let payload: Value = env.payload.clone();
        match self.pipeline.compose(payload, &mut journal) {
            Ok(result) => {
                let ws = wrap_witnesses(journal.into_witnesses());
                (true, Some(result), None, ws)
            }
            Err(e) => {
                if matches!(e, IsaError::CircuitOpen { .. }) {
                    self.governor.record_circuit_break();
                }
                let ws = wrap_witnesses(journal.into_witnesses());
                (false, None, Some(e.to_string()), ws)
            }
        }
    }
}

fn wrap_witnesses(ws: Vec<Witness>) -> Vec<OutgoingWitness> {
    ws.into_iter().map(OutgoingWitness).collect()
}

impl DynCell for TaskCell {
    fn id(&self) -> &CellId {
        &self.id
    }

    fn layer(&self) -> RuntimeTier {
        RuntimeTier::Exec
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl DynHandleCell for TaskCell {
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a> {
        Box::pin(async move {
            let (ok, result, error, witnesses) = self.handle_signal(&env, ctx);
            let raw_ws: Vec<Witness> = witnesses.iter().map(|w| w.0.clone()).collect();
            let outcome = TaskRunOutcome {
                ok,
                error: error.clone(),
                result: result.clone(),
                witnesses: raw_ws,
                governor_level: format!("{:?}", self.governor.level()),
                governor_score: self.governor.score(),
                circuit: format!("{:?}", self.pipeline.circuit_state()),
            };
            self.store_outcome(outcome);
            // Business failure is still a successful Cell handle (Witness is the audit).
            // Only panics become KernelError via dispatch.
            (Ok(()), Vec::new(), witnesses)
        })
    }
}
