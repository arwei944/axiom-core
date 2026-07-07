use axiom_kernel::axiom::{AxiomKernel, Message, State};
use axiom_kernel::cell::{CellKernel, CellKind};
use axiom_kernel::id::{CorrelationId, WitnessId};
use axiom_kernel::layer::Layer;
use axiom_kernel::lens::LensKernel;
use axiom_kernel::signal::{SignalEnvelope, SignalKernel};
use axiom_kernel::version::{SchemaVersion, VersionInfo};
use axiom_kernel::witness::{
    TransitionOutcome, Witness, WitnessHash, WitnessKernel, WitnessKind, WitnessMetrics,
};

#[tokio::test]
async fn test_cell_kernel_send_receive() {
    let kernel = CellKernel::new();
    let handle = kernel.create(CellKind::Exec).await;
    kernel
        .send(&handle, Message::new(b"hello".to_vec()))
        .await
        .unwrap();
    let received = kernel.receive(&handle).await.unwrap();
    assert_eq!(received.unwrap().payload, b"hello");
}

#[tokio::test]
async fn test_signal_kernel_send() {
    let kernel = SignalKernel::new();
    let envelope = SignalEnvelope::new(Layer::Exec, Layer::Validate, "test");
    let result = kernel.send(envelope).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_lens_kernel_query_not_found() {
    let kernel = LensKernel::new();
    let state = State::empty();
    let result = kernel.query("missing", &state).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_axiom_kernel_check() {
    let kernel = AxiomKernel::new();
    let current = State::empty();
    let new_state = State::empty();
    let msg = Message::new(Vec::new());
    let result = kernel.check(&current, &new_state, &msg).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_witness_kernel_record_and_verify() {
    let kernel = WitnessKernel::new();
    let w1 = Witness {
        witness_id: WitnessId::new("1"),
        schema_version: SchemaVersion::new(1),
        cell_id: "c1".into(),
        correlation_id: CorrelationId::new("none"),
        trace_id: None,
        triggering_msg_id: None,
        vector_clock: axiom_kernel::signal::VectorClock::new(),
        timestamp_ns: 1,
        prev_hash: Some(WitnessHash([0u8; 32])),
        state_before_hash: Some(WitnessHash([0u8; 32])),
        state_after_hash: Some(WitnessHash([0u8; 32])),
        hash: WitnessHash::zero(),
        summary: "w1".into(),
        outcome: TransitionOutcome::Success,
        metrics: WitnessMetrics::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0u8; 32],
        payload_size_bytes: 0,
        kind: WitnessKind::StateTransition,
    };
    let w2 = Witness {
        witness_id: WitnessId::new("2"),
        schema_version: SchemaVersion::new(1),
        cell_id: "c1".into(),
        correlation_id: CorrelationId::new("none"),
        trace_id: None,
        triggering_msg_id: None,
        vector_clock: axiom_kernel::signal::VectorClock::new(),
        timestamp_ns: 2,
        prev_hash: Some(WitnessHash([0u8; 32])),
        state_before_hash: Some(WitnessHash([0u8; 32])),
        state_after_hash: Some(WitnessHash([0u8; 32])),
        hash: WitnessHash::zero(),
        summary: "w2".into(),
        outcome: TransitionOutcome::Success,
        metrics: WitnessMetrics::default(),
        version_info: VersionInfo::current(),
        signal_fingerprint: [0u8; 32],
        payload_size_bytes: 0,
        kind: WitnessKind::StateTransition,
    };
    kernel.record(w1).await;
    kernel.record(w2).await;
    let result = kernel.verify_chain().await;
    assert!(result.is_ok());
}
