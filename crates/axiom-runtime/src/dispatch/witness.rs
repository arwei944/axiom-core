use axiom_kernel::id::MsgId;
use axiom_kernel::layer::Layer;
use axiom_kernel::witness::Witness;
use axiom_kernel::KernelError;
use axiom_store::Event;

pub fn witness_to_event(witness: &Witness, layer: Layer) -> Result<Event, KernelError> {
    let payload = serde_json::to_value(witness).map_err(|e| KernelError::WitnessSerialization {
        cell_id: witness.cell_id.clone(),
        message: e.to_string(),
    })?;

    let outcome = match &witness.outcome {
        axiom_kernel::witness::TransitionOutcome::Success => axiom_store::EventOutcome::Success,
        axiom_kernel::witness::TransitionOutcome::Failed { reason } => {
            axiom_store::EventOutcome::Failed {
                reason: reason.clone(),
            }
        }
        axiom_kernel::witness::TransitionOutcome::AxiomViolated {
            axiom_name,
            message,
        } => axiom_store::EventOutcome::AxiomViolated {
            axiom_name: axiom_name.clone(),
            message: message.clone(),
        },
    };

    let witness_hash_data = axiom_store::WitnessHashData {
        prev_hash: witness.prev_hash.as_ref().map(|h| h.0),
        state_before_hash: witness.state_before_hash.as_ref().map(|h| h.0),
        state_after_hash: witness.state_after_hash.as_ref().map(|h| h.0),
        hash: witness.hash.0,
        signal_fingerprint: witness.signal_fingerprint,
    };

    let event = axiom_store::EventBuilder::new(&witness.cell_id, "witness", payload)
        .event_id(witness.witness_id.as_str())
        .cell_id(&witness.cell_id)
        .correlation_id(witness.correlation_id.clone())
        .triggering_msg_id(
            witness
                .triggering_msg_id
                .clone()
                .unwrap_or_else(|| MsgId::new("unknown")),
        )
        .vector_clock(witness.vector_clock.clone())
        .layer(layer)
        .timestamp_ns(witness.timestamp_ns)
        .outcome(outcome)
        .summary(&witness.summary)
        .witness_hash(witness_hash_data)
        .payload_size_bytes(witness.payload_size_bytes)
        .build();
    Ok(event)
}
