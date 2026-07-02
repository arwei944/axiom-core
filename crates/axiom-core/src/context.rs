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
use crate::sealed::{CanSendTo, LayerMarker};
use crate::signal::{now_ns, Signal, SignalEnvelope, VectorClock};
use crate::version::SchemaVersion;
use crate::witness::{TransitionOutcome, Witness, WitnessBuilder, WitnessHash};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct OutgoingEnvelope(pub SignalEnvelope);

#[derive(Debug, Clone)]
pub struct OutgoingWitness(pub Witness);

pub struct CellContext<'a> {
    pub(crate) cell_id: &'a CellId,
    pub(crate) layer: Layer,
    pub(crate) current_msg_id: Option<MsgId>,
    pub(crate) current_correlation: Option<CorrelationId>,
    pub(crate) current_trace: Option<TraceId>,
    pub(crate) current_hop_count: Option<u32>,
    pub(crate) current_signal_type: Option<String>,
    pub(crate) current_schema_version: Option<SchemaVersion>,
    pub(crate) current_payload: Option<serde_json::Value>,
    pub(crate) last_witness_hash: Option<WitnessHash>,
    pub(crate) vector_clock: VectorClock,
    pub(crate) outgoing: Vec<OutgoingEnvelope>,
    pub(crate) witnesses: Vec<OutgoingWitness>,
    pub(crate) witness_sample_rate: f64,
    pub(crate) spawn_requests: Vec<CellSpawnRequest>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CellSpawnRequest {
    pub target_layer: Layer,
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
            vector_clock: VectorClock::new(),
            outgoing: Vec::new(),
            witnesses: Vec::new(),
            witness_sample_rate: 1.0,
            spawn_requests: Vec::new(),
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

    pub fn current_msg_id(&self) -> Option<&MsgId> {
        self.current_msg_id.as_ref()
    }

    pub fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }

    pub fn set_sample_rate(&mut self, rate: f64) {
        self.witness_sample_rate = rate.clamp(0.0, 1.0);
    }

    /// Prepare context for processing an incoming signal envelope.
    ///
    /// Sets current message metadata (msg_id, correlation, trace, hop_count,
    /// signal type/version/payload) and clears outgoing buffers.
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
        self.spawn_requests.clear();
    }

    /// Drain outgoing envelopes and witnesses after processing.
    pub fn end_processing(&mut self) -> (Vec<OutgoingEnvelope>, Vec<OutgoingWitness>) {
        let out = std::mem::take(&mut self.outgoing);
        let wit = std::mem::take(&mut self.witnesses);
        (out, wit)
    }

    fn emit_internal<S: Signal>(
        &mut self,
        signal: S,
        target_cell: Option<&str>,
        target_layer: Layer,
    ) -> crate::Result<()> {
        let validation = signal.validate();
        if validation.has_errors() {
            return Err(crate::AxiomError::SignalValidation {
                signal_type: signal.signal_type().to_string(),
                message: format!("{}", validation),
            });
        }
        if validation.has_warnings() {
            tracing::warn!(
                signal_type = signal.signal_type(),
                "signal validation produced warnings"
            );
        }

        if !self.layer.can_send_to(target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.layer,
                to: target_layer,
                signal_type: signal.signal_type().to_string(),
                source_cell: self.cell_id.as_str().to_string(),
            });
        }

        let mut env = match target_cell {
            Some(tc) => SignalEnvelope::to_cell(&signal, tc, target_layer)?,
            None => SignalEnvelope::new(&signal, target_layer)?,
        };
        env.vector_clock = self.vector_clock.clone();
        env.parent_msg_id = self.current_msg_id.clone();
        env.hop_count = self.current_hop_count.map(|h| h + 1).unwrap_or(0);
        env.source_cell = Some(self.cell_id.as_str().to_string());
        if let Some(ref corr) = self.current_correlation {
            env.correlation_id = corr.clone();
        }
        if let Some(ref trace) = self.current_trace {
            env.trace_id = Some(trace.clone());
        }
        self.outgoing.push(OutgoingEnvelope(env));
        Ok(())
    }

    pub(crate) fn send<S: Signal>(
        &mut self,
        signal: S,
        target_cell: &str,
        target_layer: Layer,
    ) -> crate::Result<()> {
        if !self.layer.can_send_to(target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.layer,
                to: target_layer,
                signal_type: signal.signal_type().to_string(),
                source_cell: self.cell_id.as_str().to_string(),
            });
        }
        self.emit_internal(signal, Some(target_cell), target_layer)
    }

    pub(crate) fn emit_event<S: Signal>(&mut self, signal: S, target_layer: Layer) -> crate::Result<()> {
        if !self.layer.can_send_to(target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.layer,
                to: target_layer,
                signal_type: signal.signal_type().to_string(),
                source_cell: self.cell_id.as_str().to_string(),
            });
        }
        self.emit_internal(signal, None, target_layer)
    }

    pub fn reply<S: Signal>(
        &mut self,
        incoming: &SignalEnvelope,
        response: S,
    ) -> crate::Result<()> {
        let target_cell = incoming.source_cell.clone().unwrap_or_default();
        let target_layer = incoming.source_layer;
        if !self.layer.can_send_to(target_layer) {
            return Err(crate::AxiomError::LayerViolation {
                from: self.layer,
                to: target_layer,
                signal_type: response.signal_type().to_string(),
                source_cell: self.cell_id.as_str().to_string(),
            });
        }
        self.emit_internal(response, Some(&target_cell), target_layer)
    }

    pub fn reply_raw<S: Signal>(&mut self, response: S) -> crate::Result<()> {
        if self.current_msg_id.is_some() {
            let target_cell = self.cell_id.as_str().to_string();
            self.emit_internal(response, Some(&target_cell), self.layer)
        } else {
            Err(crate::AxiomError::CorrelationBroken {
                message: "reply called without current message context".into(),
                correlation_id: "none".into(),
            })
        }
    }

    pub fn emit_success(&mut self, summary: &str) -> crate::Result<()> {
        let builder = self
            .witness()
            .summary(summary)
            .outcome(TransitionOutcome::Success);
        builder.emit(self)
    }

    pub fn emit_failure(&mut self, summary: &str, reason: &str) -> crate::Result<()> {
        let builder = self
            .witness()
            .summary(summary)
            .outcome(TransitionOutcome::Failed {
                reason: reason.to_string(),
            });
        builder.emit(self)
    }

    pub fn emit_axiom_violation(&mut self, axiom_name: &str, message: &str) -> crate::Result<()> {
        let builder =
            self.witness()
                .summary("axiom violation")
                .outcome(TransitionOutcome::AxiomViolated {
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

    pub fn spawn(&mut self, target_layer: Layer) -> crate::Result<CellId> {
        self.spawn_requests.push(CellSpawnRequest { target_layer });
        Ok(CellId::new(format!("spawn-{}", now_ns())))
    }

    pub fn as_layered<L: LayerMarker>(&'a mut self) -> LayeredCellContext<'a, L> {
        assert_eq!(
            self.layer,
            L::LAYER,
            "Layer mismatch: CellContext layer is {:?} but requested layer marker is {:?}",
            self.layer,
            L::LAYER
        );
        LayeredCellContext {
            inner: self,
            _marker: PhantomData,
        }
    }
}

/// Layer-specific CellContext wrapper that enforces call-direction constraints
/// at COMPILE TIME.
///
/// `LayeredCellContext<'a, L>` wraps a `CellContext` and uses the type parameter `L`
/// (a `LayerMarker`) to restrict which `send_to` and `emit_to` methods are available.
/// This prevents illegal cross-layer calls from compiling.
///
/// # Constraints by Layer
/// - `OversightLayer`: can send to Oversight, Agent, Validate, Exec
/// - `AgentLayer`: can send to Agent, Validate
/// - `ValidateLayer`: can send to Validate, Exec
/// - `ExecLayer`: can only send to Exec
///
/// # Example
/// ```
/// use axiom_core::context::LayeredCellContext;
/// use axiom_core::sealed::ExecLayer;
/// use axiom_core::id::CellId;
/// use axiom_core::layer::Layer;
///
/// // LayeredCellContext wraps a CellContext and enforces layer constraints
/// // The type parameter L (ExecLayer) restricts which send methods are available
///
/// // When implementing a Cell, the Layer type parameter determines what you can send to:
/// // - ExecLayer: can only send to ExecLayer
/// // - ValidateLayer: can send to ValidateLayer and ExecLayer
/// // - AgentLayer: can send to AgentLayer and ValidateLayer
/// // - OversightLayer: can send to all layers
/// ```
pub struct LayeredCellContext<'a, L: LayerMarker> {
    inner: &'a mut CellContext<'a>,
    _marker: PhantomData<L>,
}

impl<'a, L: LayerMarker> LayeredCellContext<'a, L> {
    /// Create a LayeredCellContext from a raw CellContext.
    ///
    /// This is typically called internally by the runtime during message dispatch.
    pub fn from_cell_context(inner: &'a mut CellContext<'a>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn cell_id(&self) -> &CellId {
        self.inner.cell_id()
    }

    pub fn layer(&self) -> Layer {
        L::LAYER
    }

    pub fn current_correlation_id(&self) -> Option<&CorrelationId> {
        self.inner.current_correlation_id()
    }

    pub fn current_msg_id(&self) -> Option<&MsgId> {
        self.inner.current_msg_id()
    }

    pub fn vector_clock(&self) -> &VectorClock {
        self.inner.vector_clock()
    }

    pub fn set_sample_rate(&mut self, rate: f64) {
        self.inner.set_sample_rate(rate);
    }

    pub fn send_to<Target: LayerMarker, S: Signal>(
        &mut self,
        signal: S,
        target_cell: &str,
    ) -> crate::Result<()>
    where
        L: CanSendTo<Target>,
    {
        self.inner.send(signal, target_cell, Target::LAYER)
    }

    pub fn emit_to<Target: LayerMarker, S: Signal>(
        &mut self,
        signal: S,
    ) -> crate::Result<()>
    where
        L: CanSendTo<Target>,
    {
        self.inner.emit_event(signal, Target::LAYER)
    }

    pub fn reply<S: Signal>(
        &mut self,
        incoming: &SignalEnvelope,
        response: S,
    ) -> crate::Result<()> {
        self.inner.reply(incoming, response)
    }

    pub fn emit_success(&mut self, summary: &str) -> crate::Result<()> {
        self.inner.emit_success(summary)
    }

    pub fn emit_failure(&mut self, summary: &str, reason: &str) -> crate::Result<()> {
        self.inner.emit_failure(summary, reason)
    }

    pub fn emit_axiom_violation(&mut self, axiom_name: &str, message: &str) -> crate::Result<()> {
        self.inner.emit_axiom_violation(axiom_name, message)
    }

    pub fn witness(&self) -> WitnessBuilder {
        self.inner.witness()
    }

    pub fn emit_witness(&mut self, builder: WitnessBuilder) -> crate::Result<()> {
        builder.emit(self.inner)
    }

    pub fn end_processing(&mut self) -> (Vec<OutgoingEnvelope>, Vec<OutgoingWitness>) {
        self.inner.end_processing()
    }

    pub fn take_outgoing(&mut self) -> Vec<OutgoingEnvelope> {
        self.inner.take_outgoing()
    }

    pub fn take_witnesses(&mut self) -> Vec<OutgoingWitness> {
        self.inner.take_witnesses()
    }
}
