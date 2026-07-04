//! Signal envelope codec for efficient serialization on the internal message bus.
//!
//! Provides a `SignalCodec` trait with JSON (default) and Bincode (optional) implementations.
//! Bincode is ~50% smaller and ~20% faster than JSON for typical `SignalEnvelope` payloads.

use crate::error::AxiomError;
use crate::signal::SignalEnvelope;
use serde_json;

/// Trait for encoding/decoding `SignalEnvelope` on the internal bus.
pub trait SignalCodec: Send + Sync + 'static {
    fn encode(&self, envelope: &SignalEnvelope) -> Result<Vec<u8>, AxiomError>;
    fn decode(&self, data: &[u8]) -> Result<SignalEnvelope, AxiomError>;
}

/// JSON-based codec (default). Human-readable but larger and slower.
#[derive(Debug, Clone, Default)]
pub struct JsonCodec;

impl SignalCodec for JsonCodec {
    fn encode(&self, envelope: &SignalEnvelope) -> Result<Vec<u8>, AxiomError> {
        serde_json::to_vec(envelope).map_err(|e| AxiomError::SignalSerialization {
            signal_type: envelope.signal_type.clone(),
            message: e.to_string(),
        })
    }

    fn decode(&self, data: &[u8]) -> Result<SignalEnvelope, AxiomError> {
        serde_json::from_slice(data).map_err(|e| AxiomError::SignalSerialization {
            signal_type: "unknown".into(),
            message: e.to_string(),
        })
    }
}

/// Bincode-based codec (feature-gated). Compact binary format.
#[derive(Debug, Clone, Default)]
#[cfg(feature = "bincode-codec")]
pub struct BincodeCodec;

#[cfg(feature = "bincode-codec")]
impl SignalCodec for BincodeCodec {
    fn encode(&self, envelope: &SignalEnvelope) -> Result<Vec<u8>, AxiomError> {
        bincode::serialize(envelope).map_err(|e| AxiomError::SignalSerialization {
            signal_type: envelope.signal_type.clone(),
            message: e.to_string(),
        })
    }

    fn decode(&self, data: &[u8]) -> Result<SignalEnvelope, AxiomError> {
        bincode::deserialize(data).map_err(|e| AxiomError::SignalSerialization {
            signal_type: "unknown".into(),
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{CorrelationId, MsgId};
    use crate::signal::{SignalKind, VectorClock};

    fn sample_envelope() -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("test-msg"),
            correlation_id: CorrelationId::new("test-corr"),
            trace_id: None,
            signal_type: "TestSignal".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1_234_567_890,
            kind: SignalKind::Command,
            source_layer: crate::Layer::Exec,
            target_layer: crate::Layer::Exec,
            source_cell: Some("cell-a".into()),
            target_cell: Some("cell-b".into()),
            payload: serde_json::json!({"hello": "world"}),
            schema_version: crate::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn json_codec_round_trip() {
        let codec = JsonCodec;
        let env = sample_envelope();
        let data = codec.encode(&env).unwrap();
        let decoded = codec.decode(&data).unwrap();
        assert_eq!(env, decoded);
    }

    #[cfg(feature = "bincode-codec")]
    #[test]
    fn bincode_codec_round_trip() {
        let codec = BincodeCodec;
        let env = sample_envelope();
        let data = codec.encode(&env).unwrap();
        let decoded = codec.decode(&data).unwrap();
        assert_eq!(env, decoded);
    }

    #[cfg(feature = "bincode-codec")]
    #[test]
    fn bincode_is_smaller_than_json() {
        let env = sample_envelope();
        let json = JsonCodec.encode(&env).unwrap();
        let bincode = BincodeCodec.encode(&env).unwrap();
        assert!(
            bincode.len() < json.len() / 2,
            "bincode should be < 50% of JSON size: json={} bincode={}",
            json.len(),
            bincode.len()
        );
    }
}
