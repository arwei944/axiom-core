use crate::id::LensId;
use crate::signal::VectorClock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensEvent {
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub sequence_number: u64,
}

pub trait Lens: Send + Sync + 'static {
    type Input: Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>;
    type Output: Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>;

    fn id(&self) -> &LensId;

    fn project(&self, events: &[LensEvent], input: &Self::Input) -> Self::Output;

    fn project_since(
        &self,
        events: &[LensEvent],
        input: &Self::Input,
        since_sequence: u64,
    ) -> Self::Output {
        let filtered: Vec<LensEvent> = events
            .iter()
            .filter(|e| e.sequence_number > since_sequence)
            .cloned()
            .collect();
        self.project(&filtered, input)
    }

    fn cache_key(&self, input: &Self::Input) -> Option<String> {
        None
    }

    fn depends_on(&self) -> &[LensId] {
        &[]
    }

    fn token_estimate(&self, output: &Self::Output) -> Option<usize> {
        None
    }

    fn summary(&self, output: &Self::Output) -> Option<String> {
        None
    }
}

pub trait Projectable: Send + Sync + 'static {
    fn id(&self) -> &LensId;

    fn project_value(&self, events: &[LensEvent], input: &serde_json::Value) -> serde_json::Value;

    fn cache_key_value(&self, input: &serde_json::Value) -> Option<String> {
        None
    }

    fn depends_on(&self) -> &[LensId] {
        &[]
    }

    fn token_estimate_value(&self, output: &serde_json::Value) -> Option<usize> {
        None
    }

    fn summary_value(&self, output: &serde_json::Value) -> Option<String> {
        None
    }

    fn input_schema(&self) -> Option<serde_json::Value> {
        None
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

impl<L: Lens> Projectable for L {
    fn id(&self) -> &LensId {
        Lens::id(self)
    }

    fn project_value(&self, events: &[LensEvent], input: &serde_json::Value) -> serde_json::Value {
        let typed_input: L::Input = serde_json::from_value(input.clone())
            .expect("Input deserialization failed");
        let typed_output = Lens::project(self, events, &typed_input);
        serde_json::to_value(typed_output)
            .expect("Output serialization failed")
    }

    fn cache_key_value(&self, input: &serde_json::Value) -> Option<String> {
        let typed_input: L::Input = serde_json::from_value(input.clone())
            .expect("Input deserialization failed");
        Lens::cache_key(self, &typed_input)
    }

    fn depends_on(&self) -> &[LensId] {
        Lens::depends_on(self)
    }

    fn token_estimate_value(&self, output: &serde_json::Value) -> Option<usize> {
        let typed_output: L::Output = serde_json::from_value(output.clone())
            .expect("Output deserialization failed");
        Lens::token_estimate(self, &typed_output)
    }

    fn summary_value(&self, output: &serde_json::Value) -> Option<String> {
        let typed_output: L::Output = serde_json::from_value(output.clone())
            .expect("Output deserialization failed");
        Lens::summary(self, &typed_output)
    }
}

#[derive(Debug, Clone)]
pub struct Projection {
    pub lens_id: LensId,
    pub input_hash: [u8; 32],
    pub output: serde_json::Value,
    pub vector_clock: VectorClock,
    pub token_count: Option<usize>,
    pub summary: Option<String>,
    pub projection_time_ms: u64,
    pub event_count: usize,
    pub was_cached: bool,
    pub last_sequence_number: Option<u64>,
}

impl Projection {
    pub fn downcast<T: serde::de::DeserializeOwned>(&self) -> Result<T, ProjectionDowncastError> {
        serde_json::from_value(self.output.clone())
            .map_err(|_| ProjectionDowncastError {
                lens_id: self.lens_id.clone(),
                expected_type: std::any::type_name::<T>().to_string(),
            })
    }

    pub fn is_within_budget(&self, max_tokens: usize) -> bool {
        self.token_count.map(|t| t <= max_tokens).unwrap_or(true)
    }

    pub fn new(
        lens_id: LensId,
        input_hash: [u8; 32],
        output: serde_json::Value,
        last_sequence_number: Option<u64>,
    ) -> Self {
        Self {
            lens_id,
            input_hash,
            output,
            vector_clock: VectorClock::new(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: 0,
            was_cached: false,
            last_sequence_number,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDowncastError {
    pub lens_id: LensId,
    pub expected_type: String,
}

impl std::error::Error for ProjectionDowncastError {}

impl std::fmt::Display for ProjectionDowncastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Projection from lens {} is not of expected type {}",
            self.lens_id, self.expected_type
        )
    }
}

pub struct LensAccessor {
    projectable: &'static dyn Projectable,
}

impl LensAccessor {
    pub fn new(projectable: &'static dyn Projectable) -> Self {
        Self { projectable }
    }

    pub fn id(&self) -> &LensId {
        self.projectable.id()
    }

    pub fn depends_on(&self) -> &[LensId] {
        self.projectable.depends_on()
    }

    pub fn project<I, O>(
        &self,
        events: &[LensEvent],
        input: &I,
    ) -> Result<Projection, LensError>
    where
        I: Serialize + Send + Sync + 'static,
        O: Deserialize<'static> + Send + Sync + 'static,
    {
        let input_value = serde_json::to_value(input)
            .map_err(|e| LensError::Serialization(e.to_string()))?;

        let output_value = self.projectable.project_value(events, &input_value);

        let token_count = self.projectable.token_estimate_value(&output_value);
        let summary = self.projectable.summary_value(&output_value);

        let input_hash = compute_hash(&input_value);

        Ok(Projection {
            lens_id: self.projectable.id().clone(),
            input_hash,
            output: output_value,
            vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
            token_count,
            summary,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        })
    }
}

pub trait ProjectionCache: Send + Sync {
    fn get_or_compute<L: Lens>(
        &self,
        lens: &L,
        events: &[LensEvent],
        input: &L::Input,
    ) -> Projection;

    fn invalidate(&self, lens_id: &LensId);

    fn invalidate_by_input_hash(&self, lens_id: &LensId, input_hash: [u8; 32]);

    fn invalidate_all(&self);

    fn metrics(&self) -> CacheMetrics;
}

#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub total_size_bytes: usize,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

pub struct InMemoryProjectionCache {
    cache: dashmap::DashMap<String, CachedProjection>,
    ttl_ms: u64,
    max_entries: usize,
    max_size_bytes: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    evictions: std::sync::atomic::AtomicU64,
}

struct CachedProjection {
    projection: Projection,
    inserted_at_ms: u64,
    size_bytes: usize,
    events: Vec<LensEvent>,
}

impl InMemoryProjectionCache {
    pub fn new() -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            ttl_ms: 30_000,
            max_entries: 10_000,
            max_size_bytes: 100_000_000,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
            evictions: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn with_ttl(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    pub fn with_max_size(mut self, max_size_bytes: usize) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }
}

impl ProjectionCache for InMemoryProjectionCache {
    fn get_or_compute<L: Lens>(
        &self,
        lens: &L,
        events: &[LensEvent],
        input: &L::Input,
    ) -> Projection {
        let key = match lens.cache_key(input) {
            Some(k) => k,
            None => {
                let output_value = serde_json::to_value(lens.project(events, input))
                    .expect("Output serialization failed");
                return Projection {
                    lens_id: lens.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
                    token_count: None,
                    summary: None,
                    projection_time_ms: 0,
                    event_count: events.len(),
                    was_cached: false,
                    last_sequence_number: events.last().map(|e| e.sequence_number),
                };
            }
        };

        if let Some(entry) = self.cache.get(&key) {
            let cached = entry.value();
            if cached.inserted_at_ms + self.ttl_ms > now_ms() {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let mut projection = cached.projection.clone();
                projection.was_cached = true;
                return projection;
            }
        }

        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let output_value = serde_json::to_value(lens.project(events, input))
            .expect("Output serialization failed");
        let projection = Projection {
            lens_id: lens.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes = std::mem::size_of::<Projection>() + projection.output.to_string().len();

        self.cache.insert(
            key,
            CachedProjection {
                projection: projection.clone(),
                inserted_at_ms: now_ms(),
                size_bytes,
                events: events.to_vec(),
            },
        );

        self.enforce_limits();

        projection
    }

    fn invalidate(&self, lens_id: &LensId) {
        self.cache.retain(|_, v| v.projection.lens_id != *lens_id);
    }

    fn invalidate_by_input_hash(&self, lens_id: &LensId, input_hash: [u8; 32]) {
        self.cache.retain(|_, v| {
            v.projection.lens_id != *lens_id || v.projection.input_hash != input_hash
        });
    }

    fn invalidate_all(&self) {
        self.cache.clear();
    }

    fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            evictions: self.evictions.load(std::sync::atomic::Ordering::Relaxed),
            entries: self.cache.len(),
            total_size_bytes: self.cache.iter().map(|e| e.value().size_bytes).sum(),
        }
    }
}

impl InMemoryProjectionCache {
    fn enforce_limits(&self) {
        while self.cache.len() > self.max_entries {
            if let Some(key) = self
                .cache
                .iter()
                .min_by_key(|e| e.value().inserted_at_ms)
                .map(|e| e.key().clone())
            {
                self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.cache.remove(&key);
            }
        }

        let mut total_size = self.cache.iter().map(|e| e.value().size_bytes).sum::<usize>();
        while total_size > self.max_size_bytes && !self.cache.is_empty() {
            if let Some(key) = self
                .cache
                .iter()
                .min_by_key(|e| e.value().inserted_at_ms)
                .map(|e| e.key().clone())
            {
                if let Some((_, removed)) = self.cache.remove(&key) {
                    self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    total_size -= removed.size_bytes;
                }
            }
        }
    }
}

pub struct IncrementalProjectionCache {
    base_projections: dashmap::DashMap<String, CachedProjection>,
    delta_events: dashmap::DashMap<String, Vec<LensEvent>>,
    ttl_ms: u64,
    max_entries: usize,
    max_size_bytes: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    evictions: std::sync::atomic::AtomicU64,
}

impl IncrementalProjectionCache {
    pub fn new() -> Self {
        Self {
            base_projections: dashmap::DashMap::new(),
            delta_events: dashmap::DashMap::new(),
            ttl_ms: 30_000,
            max_entries: 10_000,
            max_size_bytes: 100_000_000,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
            evictions: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn with_ttl(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    pub fn with_max_size(mut self, max_size_bytes: usize) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }

    pub fn on_new_event(&self, event: &LensEvent) {
        self.on_new_event_for_lenses(event, LensRegistry::registered_lenses().as_slice())
    }

    pub fn on_new_event_for_lenses(&self, event: &LensEvent, lenses: &[&'static dyn Projectable]) {
        for lens in lenses {
            if event.aggregate_id.starts_with(lens.id().as_str())
                || lens.id().as_str().starts_with(&event.aggregate_id)
            {
                let key = event.aggregate_id.clone();
                self.delta_events.entry(key).or_default().push(event.clone());
            }
        }
    }

    pub fn get_or_compute_projectable(
        &self,
        projectable: &dyn Projectable,
        events: &[LensEvent],
        input: &serde_json::Value,
    ) -> Projection {
        let key = match projectable.cache_key_value(input) {
            Some(k) => k,
            None => {
                let output_value = projectable.project_value(events, input);
                return Projection {
                    lens_id: projectable.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
                    token_count: None,
                    summary: None,
                    projection_time_ms: 0,
                    event_count: events.len(),
                    was_cached: false,
                    last_sequence_number: events.last().map(|e| e.sequence_number),
                };
            }
        };

        if let Some(base) = self.base_projections.get(&key) {
            let delta = self.delta_events.get(&key).map(|v| v.value().clone()).unwrap_or_default();
            let combined: Vec<_> = base.events.iter().chain(delta.iter()).cloned().collect();
            let output_value = projectable.project_value(&combined, input);
            let projection = Projection {
                lens_id: projectable.id().clone(),
                input_hash: compute_hash(&output_value),
                output: output_value,
                vector_clock: combined.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
                token_count: None,
                summary: None,
                projection_time_ms: 0,
                event_count: combined.len(),
                was_cached: true,
                last_sequence_number: combined.last().map(|e| e.sequence_number),
            };
            return projection;
        }

        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let output_value = projectable.project_value(events, input);
        let projection = Projection {
            lens_id: projectable.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes = std::mem::size_of::<Projection>() + projection.output.to_string().len();

        self.base_projections.insert(
            key,
            CachedProjection {
                projection: projection.clone(),
                inserted_at_ms: now_ms(),
                size_bytes,
                events: events.to_vec(),
            },
        );

        self.enforce_limits();

        projection
    }
}

impl ProjectionCache for IncrementalProjectionCache {
    fn get_or_compute<L: Lens>(
        &self,
        lens: &L,
        events: &[LensEvent],
        input: &L::Input,
    ) -> Projection {
        let key = match lens.cache_key(input) {
            Some(k) => k,
            None => {
                let output_value = serde_json::to_value(lens.project(events, input))
                    .expect("Output serialization failed");
                return Projection {
                    lens_id: lens.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
                    token_count: None,
                    summary: None,
                    projection_time_ms: 0,
                    event_count: events.len(),
                    was_cached: false,
                    last_sequence_number: events.last().map(|e| e.sequence_number),
                };
            }
        };

        if let Some(base) = self.base_projections.get(&key) {
            let delta = self.delta_events.get(&key).map(|v| v.value().clone()).unwrap_or_default();
            let combined: Vec<_> = base.events.iter().chain(delta.iter()).cloned().collect();
            let output_value = serde_json::to_value(lens.project(&combined, input))
                .expect("Output serialization failed");
            let projection = Projection {
                lens_id: lens.id().clone(),
                input_hash: compute_hash(&output_value),
                output: output_value,
                vector_clock: combined.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
                token_count: None,
                summary: None,
                projection_time_ms: 0,
                event_count: combined.len(),
                was_cached: true,
                last_sequence_number: combined.last().map(|e| e.sequence_number),
            };
            return projection;
        }

        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let output_value = serde_json::to_value(lens.project(events, input))
            .expect("Output serialization failed");
        let projection = Projection {
            lens_id: lens.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events.last().map(|e| e.vector_clock.clone()).unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes = std::mem::size_of::<Projection>() + projection.output.to_string().len();

        self.base_projections.insert(
            key,
            CachedProjection {
                projection: projection.clone(),
                inserted_at_ms: now_ms(),
                size_bytes,
                events: events.to_vec(),
            },
        );

        self.enforce_limits();

        projection
    }

    fn invalidate(&self, lens_id: &LensId) {
        self.base_projections.retain(|_, v| v.projection.lens_id != *lens_id);
        self.delta_events.retain(|k, _| !k.starts_with(lens_id.as_str()));
    }

    fn invalidate_by_input_hash(&self, lens_id: &LensId, input_hash: [u8; 32]) {
        self.base_projections.retain(|_, v| {
            v.projection.lens_id != *lens_id || v.projection.input_hash != input_hash
        });
    }

    fn invalidate_all(&self) {
        self.base_projections.clear();
        self.delta_events.clear();
    }

    fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            evictions: self.evictions.load(std::sync::atomic::Ordering::Relaxed),
            entries: self.base_projections.len(),
            total_size_bytes: self.base_projections.iter().map(|e| e.value().size_bytes).sum(),
        }
    }
}

impl IncrementalProjectionCache {
    fn enforce_limits(&self) {
        while self.base_projections.len() > self.max_entries {
            if let Some(key) = self
                .base_projections
                .iter()
                .min_by_key(|e| e.value().inserted_at_ms)
                .map(|e| e.key().clone())
            {
                self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.base_projections.remove(&key);
            }
        }

        let mut total_size = self
            .base_projections
            .iter()
            .map(|e| e.value().size_bytes)
            .sum::<usize>();
        while total_size > self.max_size_bytes && !self.base_projections.is_empty() {
            if let Some(key) = self
                .base_projections
                .iter()
                .min_by_key(|e| e.value().inserted_at_ms)
                .map(|e| e.key().clone())
            {
                if let Some((_, removed)) = self.base_projections.remove(&key) {
                    self.evictions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    total_size -= removed.size_bytes;
                }
            }
        }
    }
}

#[linkme::distributed_slice]
pub static LENS_REGISTRY: [fn() -> &'static dyn Projectable] = [..];

pub struct LensRegistry;

impl LensRegistry {
    pub fn registered_lenses() -> Vec<&'static dyn Projectable> {
        LENS_REGISTRY.iter().map(|f| f()).collect()
    }

    pub fn get_by_id(lens_id: &LensId) -> Option<&'static dyn Projectable> {
        LENS_REGISTRY.iter().find(|f| f().id() == lens_id).map(|f| f())
    }

    pub fn get_by_aggregate(aggregate_id: &str) -> Vec<&'static dyn Projectable> {
        LENS_REGISTRY
            .iter()
            .filter(|f| f().id().as_str().starts_with(aggregate_id))
            .map(|f| f())
            .collect()
    }

    pub fn validate_dependencies() -> Result<(), DependencyCycleError> {
        let graph = Self::dependency_graph();
        if let Some(cycle) = find_cycle(&graph) {
            return Err(DependencyCycleError { cycle });
        }
        Ok(())
    }

    fn dependency_graph() -> Vec<(LensId, Vec<LensId>)> {
        let mut graph = Vec::new();
        for lens in Self::registered_lenses() {
            let id = lens.id().clone();
            let deps = lens.depends_on().to_vec();
            graph.push((id, deps));
        }
        graph
    }
}

fn find_cycle(graph: &[(LensId, Vec<LensId>)]) -> Option<Vec<LensId>> {
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();
    let mut cycle = Vec::new();

    for (node, _) in graph {
        if !visited.contains(node) && dfs_cycle(graph, node, &mut visited, &mut rec_stack, &mut cycle) {
            return Some(cycle);
        }
    }
    None
}

fn dfs_cycle(
    graph: &[(LensId, Vec<LensId>)],
    node: &LensId,
    visited: &mut std::collections::HashSet<LensId>,
    rec_stack: &mut std::collections::HashSet<LensId>,
    cycle: &mut Vec<LensId>,
) -> bool {
    if !visited.contains(node) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        cycle.push(node.clone());

        if let Some((_, deps)) = graph.iter().find(|(n, _)| n == node) {
            for dep in deps {
                if !visited.contains(dep) && dfs_cycle(graph, dep, visited, rec_stack, cycle) {
                    return true;
                } else if rec_stack.contains(dep) {
                    if let Some(idx) = cycle.iter().position(|n| n == dep) {
                        cycle.drain(..idx);
                    }
                    return true;
                }
            }
        }
    }

    if let Some(idx) = cycle.iter().position(|n| n == node) {
        cycle.drain(idx..);
    }
    rec_stack.remove(node);
    false
}

#[derive(Debug, Clone)]
pub struct DependencyCycleError {
    pub cycle: Vec<LensId>,
}

impl std::error::Error for DependencyCycleError {}

impl std::fmt::Display for DependencyCycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lens dependency cycle detected: {:?}", self.cycle)
    }
}

#[derive(Debug, Error)]
pub enum LensError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Lens dependency cycle detected: {cycle:?}")]
    DependencyCycle { cycle: Vec<LensId> },

    #[error("Projection exceeds token budget: {actual} > {max}")]
    BudgetExceeded { actual: usize, max: usize },

    #[error("Axiom violation in lens {lens_id}: {axiom_name}: {message}")]
    AxiomViolation {
        lens_id: String,
        axiom_name: String,
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum LensAccessError {
    #[error("Lens not found: {lens_id}")]
    NotFound { lens_id: String },

    #[error("Cell {cell_id} is not allowed to access lens {lens_id}")]
    Forbidden { lens_id: String, cell_id: String },

    #[error("Projection type mismatch for lens {lens_id}: expected {expected}")]
    TypeMismatch { lens_id: String, expected: String },

    #[error("Projection error: {0}")]
    Projection(#[from] LensError),

    #[error("Storage error: {0}")]
    Storage(String),
}

#[cfg(feature = "sha2-id")]
fn compute_hash(value: &serde_json::Value) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).expect("Serialization failed");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(not(feature = "sha2-id"))]
fn compute_hash(_value: &serde_json::Value) -> [u8; 32] {
    [0u8; 32]
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct CustomerId(pub String);

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct OrderSummary {
        order_id: String,
        total: f64,
    }

    struct OrderSummaryLens {
        id: LensId,
    }

    impl OrderSummaryLens {
        fn new() -> Self {
            Self {
                id: LensId::from("order-summary"),
            }
        }
    }

    impl Lens for OrderSummaryLens {
        type Input = CustomerId;
        type Output = Vec<OrderSummary>;

        fn id(&self) -> &LensId {
            &self.id
        }

        fn project(&self, events: &[LensEvent], input: &CustomerId) -> Vec<OrderSummary> {
            events
                .iter()
                .filter(|e| e.aggregate_id == input.0)
                .filter(|e| e.event_type == "OrderPlaced")
                .filter_map(|e| serde_json::from_value(e.payload.clone()).ok())
                .collect()
        }

        fn cache_key(&self, input: &CustomerId) -> Option<String> {
            Some(input.0.clone())
        }
    }

    fn create_test_events(customer_id: &str, count: usize) -> Vec<LensEvent> {
        (0..count)
            .map(|i| LensEvent {
                aggregate_id: customer_id.to_string(),
                event_type: "OrderPlaced".to_string(),
                payload: serde_json::json!({
                    "order_id": format!("order-{}", i),
                    "total": (i + 1) as f64 * 100.0
                }),
                vector_clock: VectorClock::new(),
                timestamp_ns: i as u64 * 1_000_000,
                sequence_number: i as u64,
            })
            .collect()
    }

    #[test]
    fn lens_projects_events_correctly() {
        let lens = OrderSummaryLens::new();
        let events = create_test_events("customer-1", 3);
        let input = CustomerId("customer-1".to_string());

        let result = lens.project(&events, &input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].order_id, "order-0");
        assert_eq!(result[0].total, 100.0);
    }

    #[test]
    fn lens_filters_by_aggregate_id() {
        let lens = OrderSummaryLens::new();
        let mut events = create_test_events("customer-1", 2);
        events.extend(create_test_events("customer-2", 3));
        let input = CustomerId("customer-1".to_string());

        let result = lens.project(&events, &input);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn projection_downcast_works() {
        let output = serde_json::json!([{"order_id": "order-1", "total": 100.0}]);
        let projection = Projection::new(LensId::from("test"), [0u8; 32], output, None);

        let result: Vec<OrderSummary> = projection.downcast().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].order_id, "order-1");
    }

    #[test]
    fn in_memory_cache_returns_cached_result() {
        let lens = OrderSummaryLens::new();
        let cache = InMemoryProjectionCache::new();
        let events = create_test_events("customer-1", 2);
        let input = CustomerId("customer-1".to_string());

        let result1 = cache.get_or_compute(&lens, &events, &input);
        let result2 = cache.get_or_compute(&lens, &events, &input);

        assert!(!result1.was_cached);
        assert!(result2.was_cached);
        assert_eq!(result1.output, result2.output);
    }

    #[test]
    fn in_memory_cache_metrics_track_hits_misses() {
        let lens = OrderSummaryLens::new();
        let cache = InMemoryProjectionCache::new();
        let events = create_test_events("customer-1", 2);
        let input = CustomerId("customer-1".to_string());

        cache.get_or_compute(&lens, &events, &input);
        cache.get_or_compute(&lens, &events, &input);

        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
    }

    #[test]
    fn in_memory_cache_invalidate_by_lens_id() {
        let lens = OrderSummaryLens::new();
        let cache = InMemoryProjectionCache::new();
        let events = create_test_events("customer-1", 2);
        let input = CustomerId("customer-1".to_string());

        cache.get_or_compute(&lens, &events, &input);
        cache.invalidate(&LensId::from("order-summary"));

        let metrics = cache.metrics();
        assert_eq!(metrics.entries, 0);
    }

    #[test]
    fn lens_projection_works() {
        let lens = OrderSummaryLens::new();
        let cache = InMemoryProjectionCache::new();
        let events = create_test_events("customer-1", 2);
        let input = CustomerId("customer-1".to_string());

        let output = lens.project(&events, &input);
        let projection = cache.get_or_compute(&lens, &events, &input);

        assert_eq!(projection.lens_id.as_str(), "order-summary");
        let result: Vec<OrderSummary> = projection.downcast().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result, output);
    }

    #[test]
    fn projection_is_within_budget() {
        let projection = Projection {
            lens_id: LensId::from("test"),
            input_hash: [0u8; 32],
            output: serde_json::json!("test"),
            vector_clock: VectorClock::new(),
            token_count: Some(10),
            summary: None,
            projection_time_ms: 0,
            event_count: 0,
            was_cached: false,
            last_sequence_number: None,
        };

        assert!(projection.is_within_budget(100));
        assert!(!projection.is_within_budget(5));
    }

    #[test]
    fn lens_registry_get_by_id_returns_none_for_unknown_id() {
        let result = LensRegistry::get_by_id(&LensId::from("unknown-lens"));
        assert!(result.is_none());
    }

    #[test]
    fn dependency_cycle_detection_works() {
        let result = LensRegistry::validate_dependencies();
        assert!(result.is_ok());
    }
}