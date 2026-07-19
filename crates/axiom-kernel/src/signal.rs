use crate::axiom::{KernelResult, ValidationResult};
use crate::id::{CorrelationId, MsgId, TraceId};
use crate::layer::RuntimeTier;
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
    pub source_layer: crate::RuntimeTier,
    pub target_layer: crate::RuntimeTier,
    pub source_cell: Option<String>,
    pub target_cell: Option<String>,
    pub payload: serde_json::Value,
    pub schema_version: crate::version::SchemaVersion,
    pub parent_msg_id: Option<crate::id::MsgId>,
    pub hop_count: u32,
}

impl SignalEnvelope {
    pub fn new(
        source_layer: crate::RuntimeTier,
        target_layer: crate::RuntimeTier,
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

    /// Single authority: delegates to [`RuntimeTier::can_send_to`].
    pub fn validate_layer_transition(&self) -> crate::KernelResult<()> {
        if self.source_layer.can_send_to(self.target_layer) {
            Ok(())
        } else {
            Err(crate::KernelError::LayerViolation {
                from: self.source_layer,
                to: self.target_layer,
                signal_type: self.signal_type.clone(),
                source_cell: self.source_cell.clone().unwrap_or_default(),
            })
        }
    }
}

/// Bounded LRU cache entry for SignalKernel short-circuit (P2-2).
#[derive(Clone)]
struct CacheEntry {
    fingerprint: [u8; 32],
    inserted_ns: u64,
}

pub struct SignalKernel {
    handlers: RwLock<Vec<crate::axiom::BoxedSignalHandler>>,
    heatmap: std::sync::Arc<RwLock<HeatmapCollector>>,
    /// signal_type -> last successful envelope fingerprint
    cache: RwLock<std::collections::HashMap<String, CacheEntry>>,
    cache_capacity: usize,
    cache_ttl_ns: u64,
}

impl SignalKernel {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            heatmap: std::sync::Arc::new(RwLock::new(HeatmapCollector::new())),
            cache: RwLock::new(std::collections::HashMap::new()),
            cache_capacity: 256,
            cache_ttl_ns: 5_000_000_000, // 5s
        }
    }

    pub fn with_heatmap(heatmap: std::sync::Arc<RwLock<HeatmapCollector>>) -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            heatmap,
            cache: RwLock::new(std::collections::HashMap::new()),
            cache_capacity: 256,
            cache_ttl_ns: 5_000_000_000,
        }
    }

    pub fn with_cache_limits(mut self, capacity: usize, ttl_ns: u64) -> Self {
        self.cache_capacity = capacity.max(1);
        self.cache_ttl_ns = ttl_ns;
        self
    }

    pub fn heatmap(&self) -> std::sync::Arc<RwLock<HeatmapCollector>> {
        self.heatmap.clone()
    }

    pub async fn cache_len(&self) -> usize {
        self.cache.read().await.len()
    }

    fn fingerprint(env: &SignalEnvelope) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        env.signal_type.hash(&mut h);
        env.msg_id.as_str().hash(&mut h);
        env.payload.to_string().hash(&mut h);
        let v = h.finish();
        let mut out = [0u8; 32];
        out[..8].copy_from_slice(&v.to_be_bytes());
        out
    }

    pub async fn send(&self, mut envelope: SignalEnvelope) -> KernelResult<SignalEnvelope> {
        envelope.validate_layer_transition()?;
        let fp = Self::fingerprint(&envelope);
        let now = crate::clock::global_clock().now_ns();
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&envelope.signal_type) {
                if entry.fingerprint == fp
                    && now.saturating_sub(entry.inserted_ns) < self.cache_ttl_ns
                {
                    self.heatmap
                        .write()
                        .await
                        .record_signal_send(envelope.signal_type.clone());
                    return Ok(envelope);
                }
            }
        }

        let mut handlers = self.handlers.write().await;
        for handler in handlers.iter_mut() {
            handler.handle(&mut envelope)?;
        }
        drop(handlers);

        {
            let mut cache = self.cache.write().await;
            if cache.len() >= self.cache_capacity {
                // drop arbitrary oldest-ish: clear half
                let keys: Vec<_> = cache.keys().take(cache.len() / 2).cloned().collect();
                for k in keys {
                    cache.remove(&k);
                }
            }
            cache.insert(
                envelope.signal_type.clone(),
                CacheEntry {
                    fingerprint: fp,
                    inserted_ns: now,
                },
            );
        }

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

#[cfg(test)]
mod layer_tests {
    use super::*;
    use crate::id::{CorrelationId, MsgId};
    use crate::layer::RuntimeTier;

    fn env(from: RuntimeTier, to: RuntimeTier) -> SignalEnvelope {
        SignalEnvelope {
            msg_id: MsgId::new("m"),
            correlation_id: CorrelationId::new("c"),
            trace_id: None,
            signal_type: "T".into(),
            vector_clock: VectorClock::new(),
            timestamp_ns: 1,
            kind: SignalKind::Command,
            source_layer: from,
            target_layer: to,
            source_cell: None,
            target_cell: None,
            payload: serde_json::Value::Null,
            schema_version: SchemaVersion::new(1),
            parent_msg_id: None,
            hop_count: 0,
        }
    }

    #[test]
    fn layer_validation_matches_can_send_to() {
        for from in [
            RuntimeTier::Oversight,
            RuntimeTier::Agent,
            RuntimeTier::Validate,
            RuntimeTier::Exec,
        ] {
            for to in [
                RuntimeTier::Oversight,
                RuntimeTier::Agent,
                RuntimeTier::Validate,
                RuntimeTier::Exec,
            ] {
                let e = env(from, to);
                let ok = e.validate_layer_transition().is_ok();
                assert_eq!(ok, from.can_send_to(to), "{from:?} -> {to:?}");
            }
        }
    }

    #[tokio::test]
    async fn lru_cache_bounded() {
        let sk = SignalKernel::new().with_cache_limits(4, u64::MAX);
        for i in 0..20 {
            let mut e = env(RuntimeTier::Exec, RuntimeTier::Exec);
            e.signal_type = format!("T{i}");
            e.msg_id = MsgId::new(format!("m{i}"));
            sk.send(e).await.unwrap();
        }
        assert!(sk.cache_len().await <= 4);
    }
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
    fn layer(&self) -> RuntimeTier;
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
