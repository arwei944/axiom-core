use crate::axiom::{KernelResult, ValidationResult};
use crate::id::{CorrelationId, MsgId, TraceId};
use crate::layer::Layer;
use crate::version::SchemaVersion;
use crate::HeatmapCollector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock(pub HashMap<String, u64>);

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&mut self, cell_id: &str) {
        *self.0.entry(cell_id.to_string()).or_insert(0) += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (key, value) in &other.0 {
            let entry = self.0.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    pub fn causally_precedes(&self, other: &VectorClock) -> bool {
        for (key, &self_val) in &self.0 {
            match other.0.get(key) {
                Some(&other_val) if self_val > other_val => return false,
                None if self_val > 0 => return false,
                _ => {}
            }
        }
        true
    }

    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.causally_precedes(other) && !other.causally_precedes(self)
    }

    pub fn get(&self, cell_id: &str) -> u64 {
        self.0.get(cell_id).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Command,
    Event,
    Query,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalEnvelope {
    pub msg_id: crate::id::MsgId,
    pub correlation_id: crate::id::CorrelationId,
    pub trace_id: Option<crate::id::TraceId>,
    pub signal_type: String,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub kind: SignalKind,
    pub source_layer: crate::Layer,
    pub target_layer: crate::Layer,
    pub source_cell: Option<String>,
    pub target_cell: Option<String>,
    pub payload: serde_json::Value,
    pub schema_version: crate::version::SchemaVersion,
    pub parent_msg_id: Option<crate::id::MsgId>,
    pub hop_count: u32,
}

impl SignalEnvelope {
    pub fn new(
        source_layer: crate::Layer,
        target_layer: crate::Layer,
        signal_type: impl Into<String>,
    ) -> Self {
        Self {
            msg_id: crate::id::MsgId::new(Uuid::new_v4().to_string()),
            correlation_id: crate::id::CorrelationId::new("kernel"),
            trace_id: None,
            signal_type: signal_type.into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: crate::clock::global_clock().now_ns(),
            kind: SignalKind::Command,
            source_layer,
            target_layer,
            source_cell: None,
            target_cell: None,
            payload: serde_json::Value::Null,
            schema_version: crate::version::SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    pub fn validate_layer_transition(&self) -> crate::KernelResult<()> {
        use crate::Layer::*;
        match (self.source_layer, self.target_layer) {
            (Exec, Agent) => Err(crate::KernelError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
                source_cell: self.source_cell.clone().unwrap_or_default(),
            }),
            (Exec, Oversight) => Err(crate::KernelError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
                source_cell: self.source_cell.clone().unwrap_or_default(),
            }),
            (Validate, Agent) => Err(crate::KernelError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
                source_cell: self.source_cell.clone().unwrap_or_default(),
            }),
            _ => Ok(()),
        }
    }
}

pub struct SignalKernel {
    handlers: RwLock<Vec<crate::axiom::BoxedSignalHandler>>,
    heatmap: std::sync::Arc<RwLock<HeatmapCollector>>,
}

impl SignalKernel {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            heatmap: std::sync::Arc::new(RwLock::new(HeatmapCollector::new())),
        }
    }

    pub fn with_heatmap(heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            heatmap,
        }
    }

    pub fn heatmap(&self) -> std::sync::Arc<RwLock<HeatmapCollector>> {
        self.heatmap.clone()
    }

    pub async fn send(&self, mut envelope: SignalEnvelope) -> KernelResult<SignalEnvelope> {
        let mut handlers = self.handlers.write().await;
        for handler in handlers.iter_mut() {
            handler.handle(&mut envelope)?;
        }
        drop(handlers);
        self.heatmap
            .write()
            .await
            .record_signal_send(envelope.signal_type.clone());
        Ok(envelope)
    }

    pub async fn register_handler(&self, handler: crate::axiom::BoxedSignalHandler) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }
}

impl Default for SignalKernel {
    fn default() -> Self {
        Self::new()
    }
}

pub fn now_ns() -> u64 {
    crate::clock::global_clock().now_ns()
}

pub trait Signal: Send + Sync {
    fn signal_type(&self) -> &'static str;
    fn msg_id(&self) -> &MsgId;
    fn correlation_id(&self) -> &CorrelationId;
    fn trace_id(&self) -> Option<&TraceId> {
        None
    }
    fn vector_clock(&self) -> &VectorClock;
    fn timestamp_ns(&self) -> u64;
    fn kind(&self) -> SignalKind;
    fn layer(&self) -> Layer;
    fn sender(&self) -> Option<&str> {
        None
    }
    fn schema_version(&self) -> SchemaVersion {
        SchemaVersion::new(1)
    }
    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_signal(&self) -> Box<dyn Signal>;
    fn validate(&self) -> ValidationResult;
    fn serialize_to_json(&self) -> KernelResult<serde_json::Value>;
}
