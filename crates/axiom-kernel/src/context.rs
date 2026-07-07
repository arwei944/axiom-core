use crate::axiom::KernelResult;
use crate::id::CellId;
use crate::layer::Layer;
use crate::signal::SignalEnvelope;
use crate::version::SchemaVersion;
use crate::witness::{TransitionOutcome, Witness, WitnessBuilder, WitnessHash};

#[derive(Debug, Clone)]
pub struct OutgoingEnvelope(pub SignalEnvelope);

#[derive(Debug, Clone)]
pub struct OutgoingWitness(pub Witness);

pub struct CellContext<'a> {
    pub(crate) cell_id: &'a CellId,
    #[allow(dead_code)]
    pub(crate) layer: Layer,
    pub(crate) current_msg_id: Option<crate::id::MsgId>,
    pub(crate) current_correlation: Option<crate::id::CorrelationId>,
    pub(crate) current_trace: Option<crate::id::TraceId>,
    pub(crate) current_hop_count: Option<u32>,
    pub(crate) current_signal_type: Option<String>,
    pub(crate) current_schema_version: Option<SchemaVersion>,
    pub(crate) current_payload: Option<serde_json::Value>,
    pub(crate) last_witness_hash: Option<WitnessHash>,
    pub(crate) vector_clock: crate::signal::VectorClock,
    pub(crate) outgoing: Vec<OutgoingEnvelope>,
    pub(crate) witnesses: Vec<OutgoingWitness>,
    #[allow(dead_code)]
    pub(crate) witness_sample_rate: f64,
}

impl<'a> CellContext<'a> {
    pub fn new(cell_id: &'a CellId, layer: Layer) -> Self {
        Self {
            cell_id,
            layer,
            current_msg_id: None,
            current_correlation: None,
            current_trace: None,
            current_hop_count: None,
            current_signal_type: None,
            current_schema_version: None,
            current_payload: None,
            last_witness_hash: None,
            vector_clock: crate::signal::VectorClock::new(),
            outgoing: Vec::new(),
            witnesses: Vec::new(),
            witness_sample_rate: 1.0,
        }
    }

    pub fn begin_processing(&mut self, env: &SignalEnvelope) {
        self.current_msg_id = Some(env.msg_id.clone());
        self.current_correlation = Some(env.correlation_id.clone());
        self.current_trace = env.trace_id.clone();
        self.current_hop_count = Some(env.hop_count);
        self.current_signal_type = Some(env.signal_type.clone());
        self.current_schema_version = Some(env.schema_version);
        self.current_payload = Some(env.payload.clone());
        self.vector_clock.merge(&env.vector_clock);
        self.vector_clock.increment(self.cell_id.as_str());
        self.outgoing.clear();
        self.witnesses.clear();
    }

    pub fn emit_success(&mut self, summary: &str) -> KernelResult<()> {
        let builder = self.witness().summary(summary).outcome(TransitionOutcome::Success);
        builder.emit(self)
    }

    pub fn emit_failure(&mut self, summary: &str, reason: &str) -> KernelResult<()> {
        let builder = self
            .witness()
            .summary(summary)
            .outcome(TransitionOutcome::Failed { reason: reason.to_string() });
        builder.emit(self)
    }

    pub fn emit_axiom_violation(&mut self, axiom_name: &str, message: &str) -> KernelResult<()> {
        let builder =
            self.witness().summary("axiom violation").outcome(TransitionOutcome::AxiomViolated {
                axiom_name: axiom_name.to_string(),
                message: message.to_string(),
            });
        builder.emit(self)
    }

    pub fn witness(&self) -> WitnessBuilder {
        WitnessBuilder::new()
    }

    pub fn take_outgoing(&mut self) -> Vec<OutgoingEnvelope> {
        std::mem::take(&mut self.outgoing)
    }

    pub fn take_witnesses(&mut self) -> Vec<OutgoingWitness> {
        std::mem::take(&mut self.witnesses)
    }
}
