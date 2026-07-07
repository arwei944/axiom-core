use crate::KernelError;
use crate::KernelResult;

pub trait SignalCodec: Send + Sync + 'static {
    fn encode(&self, envelope: &crate::signal::SignalEnvelope) -> KernelResult<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> KernelResult<crate::signal::SignalEnvelope>;
}

#[derive(Debug, Clone, Default)]
pub struct JsonCodec;

impl SignalCodec for JsonCodec {
    fn encode(&self, envelope: &crate::signal::SignalEnvelope) -> KernelResult<Vec<u8>> {
        serde_json::to_vec(envelope).map_err(|e| KernelError::SerializationError(e.to_string()))
    }

    fn decode(&self, data: &[u8]) -> KernelResult<crate::signal::SignalEnvelope> {
        serde_json::from_slice(data).map_err(|e| KernelError::SerializationError(e.to_string()))
    }
}

#[cfg(feature = "bincode-codec")]
#[derive(Debug, Clone, Default)]
pub struct BincodeCodec;

#[cfg(feature = "bincode-codec")]
impl SignalCodec for BincodeCodec {
    fn encode(&self, envelope: &crate::signal::SignalEnvelope) -> KernelResult<Vec<u8>> {
        bincode::serialize(envelope).map_err(|e| KernelError::SerializationError(e.to_string()))
    }

    fn decode(&self, data: &[u8]) -> KernelResult<crate::signal::SignalEnvelope> {
        bincode::deserialize(data).map_err(|e| KernelError::SerializationError(e.to_string()))
    }
}
