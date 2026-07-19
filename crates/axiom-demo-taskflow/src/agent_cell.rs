//! Agent-tier Cell: Handoff Signal → Governor admit → Workbench → Witness.

use crate::run_log::{push_run, RunSummary, SharedRunLog};
use crate::workbench::workbench_composer;
use axiom_isa::{
    product_admit, Composer, Governor, GovernorConfig, HandoffRequest, HandoffResult, SeqComposer,
    WitnessJournal,
};
use axiom_kernel::cell::{BoxHandleFuture, DynCell, DynHandleCell};
use axiom_kernel::context::{CellContext, OutgoingWitness};
use axiom_kernel::id::CellId;
use axiom_kernel::signal::SignalEnvelope;
use axiom_kernel::witness::Witness;
use axiom_kernel::RuntimeTier;
use std::sync::{Arc, Mutex};

pub const AGENT_CELL_ID: &str = "agent-cell";
pub const SIGNAL_HANDOFF: &str = "AgentHandoff";

#[derive(Debug, Clone)]
pub struct AgentRunOutcome {
    pub ok: bool,
    pub error: Option<String>,
    pub result: Option<HandoffResult>,
    pub witnesses: Vec<Witness>,
    pub governor_level: String,
    pub governor_score: f64,
}

pub type SharedAgentOutcome = Arc<Mutex<Option<AgentRunOutcome>>>;

pub fn new_shared_agent_outcome() -> SharedAgentOutcome {
    Arc::new(Mutex::new(None))
}

pub struct AgentCell {
    id: CellId,
    governor: Governor,
    workbench: SeqComposer<HandoffRequest, HandoffResult>,
    last: SharedAgentOutcome,
    runs: SharedRunLog,
}

impl AgentCell {
    pub fn new(governor: Governor, last: SharedAgentOutcome, runs: SharedRunLog) -> Self {
        Self {
            id: CellId::new(AGENT_CELL_ID),
            governor,
            workbench: workbench_composer(),
            last,
            runs,
        }
    }

    pub fn with_config(
        config: GovernorConfig,
        last: SharedAgentOutcome,
        runs: SharedRunLog,
    ) -> Self {
        Self::new(Governor::with_config(config), last, runs)
    }

    pub fn trip_governor(&mut self) {
        self.governor.trip();
    }

    pub fn preload_entropy(&mut self) {
        self.governor.record_circuit_break();
    }

    fn store(&self, outcome: AgentRunOutcome) {
        push_run(
            &self.runs,
            RunSummary {
                kind: "handoff".into(),
                ok: outcome.ok,
                label: outcome
                    .result
                    .as_ref()
                    .map(|r| r.token.clone())
                    .unwrap_or_else(|| "handoff".into()),
                governor_level: outcome.governor_level.clone(),
                governor_score: outcome.governor_score,
                witness_count: outcome.witnesses.len(),
                error: outcome.error.clone(),
            },
        );
        if let Ok(mut g) = self.last.lock() {
            *g = Some(outcome);
        }
    }

    fn handle_signal<'a>(
        &mut self,
        env: &SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> (bool, Option<HandoffResult>, Option<String>, Vec<OutgoingWitness>) {
        let mut journal = WitnessJournal::new(ctx);

        if env.signal_type != SIGNAL_HANDOFF {
            let msg = format!("unsupported signal_type {}", env.signal_type);
            let _ = journal.record_err(axiom_isa::StepKind::Composer, "agent-cell", &msg);
            return (false, None, Some(msg), wrap(journal.into_witnesses()));
        }

        // Sole product admit API
        if let Err(e) = product_admit(&self.governor, &mut journal) {
            self.governor.record_rejected();
            return (false, None, Some(e.to_string()), wrap(journal.into_witnesses()));
        }

        let req: HandoffRequest = match serde_json::from_value(env.payload.clone()) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("invalid HandoffRequest: {e}");
                let _ = journal.record_err(axiom_isa::StepKind::Adapter, "parse_handoff", &msg);
                return (false, None, Some(msg), wrap(journal.into_witnesses()));
            }
        };

        match self.workbench.compose(req, &mut journal) {
            Ok(result) => (true, Some(result), None, wrap(journal.into_witnesses())),
            Err(e) => (false, None, Some(e.to_string()), wrap(journal.into_witnesses())),
        }
    }
}

fn wrap(ws: Vec<Witness>) -> Vec<OutgoingWitness> {
    ws.into_iter().map(OutgoingWitness).collect()
}

impl DynCell for AgentCell {
    fn id(&self) -> &CellId {
        &self.id
    }

    fn layer(&self) -> RuntimeTier {
        RuntimeTier::Agent
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl DynHandleCell for AgentCell {
    fn handle_dyn<'a>(
        &'a mut self,
        env: SignalEnvelope,
        ctx: &'a mut CellContext<'a>,
    ) -> BoxHandleFuture<'a> {
        Box::pin(async move {
            let (ok, result, error, witnesses) = self.handle_signal(&env, ctx);
            let raw: Vec<Witness> = witnesses.iter().map(|w| w.0.clone()).collect();
            let outcome = AgentRunOutcome {
                ok,
                error: error.clone(),
                result: result.clone(),
                witnesses: raw,
                governor_level: format!("{:?}", self.governor.level()),
                governor_score: self.governor.score(),
            };
            self.store(outcome);
            (Ok(()), Vec::new(), witnesses)
        })
    }
}
