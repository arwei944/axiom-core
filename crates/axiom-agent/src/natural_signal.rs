use axiom_kernel::id::{CorrelationId, MsgId, TraceId};
use axiom_kernel::signal::{Signal, SignalKind, VectorClock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalSignal {
    pub msg_id: MsgId,
    pub correlation_id: CorrelationId,
    pub trace_id: Option<TraceId>,
    pub vector_clock: VectorClock,
    pub intent: String,
    pub confidence: f64,
    pub entities: Vec<Entity>,
    pub context: HashMap<String, String>,
    pub content: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub r#type: String,
    pub value: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub r#type: String,
    pub content: String,
}

impl Signal for NaturalSignal {
    fn signal_type(&self) -> &'static str {
        "NaturalSignal"
    }

    fn msg_id(&self) -> &MsgId {
        &self.msg_id
    }

    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    fn vector_clock(&self) -> &VectorClock {
        &self.vector_clock
    }

    fn timestamp_ns(&self) -> u64 {
        0
    }

    fn kind(&self) -> SignalKind {
        SignalKind::Command
    }

    fn layer(&self) -> axiom_kernel::Layer {
        axiom_kernel::Layer::Agent
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_signal(&self) -> Box<dyn Signal> {
        Box::new(self.clone())
    }

    fn validate(&self) -> axiom_kernel::axiom::ValidationResult {
        axiom_kernel::axiom::ValidationResult::ok()
    }

    fn serialize_to_json(&self) -> axiom_kernel::KernelResult<serde_json::Value> {
        serde_json::to_value(self)
            .map_err(|e| axiom_kernel::KernelError::SerializationError(e.to_string()))
    }
}

impl NaturalSignal {
    pub fn new(content: &str) -> Self {
        Self {
            msg_id: MsgId::new(uuid::Uuid::new_v4().to_string()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4().to_string()),
            trace_id: None,
            vector_clock: VectorClock::new(),
            intent: "unknown".to_string(),
            confidence: 0.0,
            entities: Vec::new(),
            context: HashMap::new(),
            content: content.to_string(),
            attachments: Vec::new(),
        }
    }

    pub fn with_intent(mut self, intent: &str, confidence: f64) -> Self {
        self.intent = intent.to_string();
        self.confidence = confidence;
        self
    }

    pub fn with_entity(mut self, name: &str, r#type: &str, value: &str, confidence: f64) -> Self {
        self.entities.push(Entity {
            name: name.to_string(),
            r#type: r#type.to_string(),
            value: value.to_string(),
            confidence,
        });
        self
    }

    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_trace(mut self, trace: TraceId) -> Self {
        self.trace_id = Some(trace);
        self
    }

    pub fn with_attachment(mut self, name: &str, r#type: &str, content: &str) -> Self {
        self.attachments.push(Attachment {
            name: name.to_string(),
            r#type: r#type.to_string(),
            content: content.to_string(),
        });
        self
    }
}