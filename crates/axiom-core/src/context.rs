//! CellContext - Runtime context provided to each Cell during message handling.
//!
//! CellContext is the ONLY way a Cell can interact with the outside world:
//! - Send messages to other Cells (layer-restricted at compile time)
//! - Produce Witness records
//! - Emit events to the event store
//!
//! Layer-specific send methods enforce the architecture call-direction rule
//! at COMPILE TIME. ExecCellContext can only send to Exec. AgentCellContext
//! can send to Agent and Validate. Etc.

use crate::id::{CellId, CorrelationId, MsgId, TraceId};
use crate::layer::Layer;
use crate::signal::{Signal, SignalEnvelope, VectorClock};
use crate::witness::{Witness, WitnessBuilder};

/// Outgoing signal envelope queued for delivery.
#[derive(Debug, Clone)]
pub struct OutgoingEnvelope(pub SignalEnvelope);

/// Outgoing witness record.
#[derive(Debug, Clone)]
pub struct OutgoingWitness(pub Witness);

/// The base CellContext used internally by the runtime.
/// Layer-specific wrapper types provide compile-time send restrictions.
pub struct CellContext<'a> {
    pub(crate) cell_id: &'a CellId,
    pub(crate) layer: Layer,
    pub(crate) current_msg_id: Option<MsgId>,
    pub(crate) current_correlation: Option<CorrelationId>,
    pub(crate) current_trace: Option<TraceId>,
    pub(crate) vector_clock: VectorClock,
    pub(crate) outgoing: Vec<OutgoingEnvelope>,
    pub(crate) witnesses: Vec<OutgoingWitness>,
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
            vector_clock: VectorClock::new(),
            outgoing: Vec::new(),
            witnesses: Vec::new(),
            witness_sample_rate: 1.0,
        }
    }

    pub fn cell_id(&self) -> &CellId {
        self.cell_id
    }

    pub fn layer(&self) -> Layer {
        self.layer
    }

    pub fn current_correlation_id(&self) -> Option<&CorrelationId> {
        self.current_correlation.as_ref()
    }

    pub fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }

    pub fn set_sample_rate(&mut self, rate: f64) {
        self.witness_sample_rate = rate.clamp(0.0, 1.0);
    }

    #[allow(dead_code)]
    pub(crate) fn begin_processing(&mut self, env: &SignalEnvelope) {
        self.current_msg_id = Some(env.msg_id.clone());
        self.current_correlation = Some(env.correlation_id.clone());
        self.current_trace = env.trace_id.clone();
        self.vector_clock.merge(&env.vector_clock);
        self.vector_clock.increment(self.cell_id.as_str());
        self.outgoing.clear();
        self.witnesses.clear();
    }

    #[allow(dead_code)]
    pub(crate) fn end_processing(&mut self) -> (Vec<OutgoingEnvelope>, Vec<OutgoingWitness>) {
        let out = std::mem::take(&mut self.outgoing);
        let wit = std::mem::take(&mut self.witnesses);
        (out, wit)
    }

    fn emit_internal<S: Signal>(
        &mut self,
        signal: S,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) {
        if !self.layer.can_send_to(target_layer) {
            tracing::error!(
                "Layer violation: {} cannot send to {} (signal: {})",
                self.layer,
                target_layer,
                signal.signal_type()
            );
            return;
        }
        let mut env = match target_cell {
            Some(tc) => SignalEnvelope::to_cell(&signal, tc, target_layer),
            None => SignalEnvelope::new(&signal, target_layer),
        };
        env.vector_clock = self.vector_clock.clone();
        env.parent_msg_id = self.current_msg_id.clone();
        env.hop_count = 0;
        if let Some(ref corr) = self.current_correlation {
            env.correlation_id = corr.clone();
        }
        if let Some(ref trace) = self.current_trace {
            env.trace_id = Some(trace.clone());
        }
        self.outgoing.push(OutgoingEnvelope(env));
    }

    /// Internal send to any layer - only used by Oversight and runtime internals.
    pub fn send_any<S: Signal>(&mut self, signal: S, target_cell: &str, target_layer: Layer) {
        self.emit_internal(signal, Some(target_cell), target_layer);
    }

    pub fn emit_to_layer<S: Signal>(&mut self, signal: S, target_layer: Layer) {
        self.emit_internal(signal, None, target_layer);
    }

    pub fn witness(&self) -> WitnessBuilder {
        WitnessBuilder::new()
    }

    pub(crate) fn add_witness(&mut self, witness: Witness) {
        let should_sample = self.witness_sample_rate >= 1.0
            || (self.witness_sample_rate > 0.0 && {
                let r: f64 = {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    witness.witness_id.as_str().hash(&mut h);
                    (h.finish() as f64) / (u64::MAX as f64)
                };
                r < self.witness_sample_rate
            });
        if should_sample {
            self.witnesses.push(OutgoingWitness(witness));
        }
    }

    pub fn take_outgoing(&mut self) -> Vec<OutgoingEnvelope> {
        std::mem::take(&mut self.outgoing)
    }

    pub fn take_witnesses(&mut self) -> Vec<OutgoingWitness> {
        std::mem::take(&mut self.witnesses)
    }
}

/// Layer-specific CellContext wrappers that enforce send restrictions at compile time.
pub struct ExecCellContext<'a>(pub(crate) &'a mut CellContext<'a>);

impl<'a> ExecCellContext<'a> {
    pub fn cell_id(&self) -> &CellId {
        self.0.cell_id()
    }
    pub fn vector_clock(&self) -> &VectorClock {
        self.0.vector_clock()
    }

    pub fn send_to_exec<S: Signal>(&mut self, signal: S, target_cell: &str) {
        self.0.emit_internal(signal, Some(target_cell), Layer::Exec);
    }

    pub fn emit_exec_event<S: Signal>(&mut self, signal: S) {
        self.0.emit_internal(signal, None, Layer::Exec);
    }

    pub fn witness(&self) -> WitnessBuilder {
        self.0.witness()
    }
}

pub struct ValidateCellContext<'a>(pub(crate) &'a mut CellContext<'a>);

impl<'a> ValidateCellContext<'a> {
    pub fn cell_id(&self) -> &CellId {
        self.0.cell_id()
    }
    pub fn vector_clock(&self) -> &VectorClock {
        self.0.vector_clock()
    }

    pub fn send_to_validate<S: Signal>(&mut self, signal: S, target_cell: &str) {
        self.0
            .emit_internal(signal, Some(target_cell), Layer::Validate);
    }

    pub fn send_to_exec<S: Signal>(&mut self, signal: S, target_cell: &str) {
        self.0.emit_internal(signal, Some(target_cell), Layer::Exec);
    }

    pub fn emit_validate_event<S: Signal>(&mut self, signal: S) {
        self.0.emit_internal(signal, None, Layer::Validate);
    }

    pub fn emit_exec_event<S: Signal>(&mut self, signal: S) {
        self.0.emit_internal(signal, None, Layer::Exec);
    }

    pub fn witness(&self) -> WitnessBuilder {
        self.0.witness()
    }
}

pub struct AgentCellContext<'a>(pub(crate) &'a mut CellContext<'a>);

impl<'a> AgentCellContext<'a> {
    pub fn cell_id(&self) -> &CellId {
        self.0.cell_id()
    }
    pub fn vector_clock(&self) -> &VectorClock {
        self.0.vector_clock()
    }

    pub fn send_to_agent<S: Signal>(&mut self, signal: S, target_cell: &str) {
        self.0
            .emit_internal(signal, Some(target_cell), Layer::Agent);
    }

    pub fn send_to_validate<S: Signal>(&mut self, signal: S, target_cell: &str) {
        self.0
            .emit_internal(signal, Some(target_cell), Layer::Validate);
    }

    pub fn emit_agent_event<S: Signal>(&mut self, signal: S) {
        self.0.emit_internal(signal, None, Layer::Agent);
    }

    pub fn emit_validate_event<S: Signal>(&mut self, signal: S) {
        self.0.emit_internal(signal, None, Layer::Validate);
    }

    pub fn witness(&self) -> WitnessBuilder {
        self.0.witness()
    }
}

pub struct OversightCellContext<'a>(pub(crate) &'a mut CellContext<'a>);

impl<'a> OversightCellContext<'a> {
    pub fn cell_id(&self) -> &CellId {
        self.0.cell_id()
    }
    pub fn vector_clock(&self) -> &VectorClock {
        self.0.vector_clock()
    }

    pub fn send_any<S: Signal>(&mut self, signal: S, target_cell: &str, target_layer: Layer) {
        self.0.send_any(signal, target_cell, target_layer);
    }

    pub fn emit_event<S: Signal>(&mut self, signal: S, target_layer: Layer) {
        self.0.emit_to_layer(signal, target_layer);
    }

    pub fn witness(&self) -> WitnessBuilder {
        self.0.witness()
    }
}
