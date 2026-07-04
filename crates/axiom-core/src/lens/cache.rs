use super::events::LensEvent;
use super::traits::Lens;
use crate::id::LensId;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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
    cache: DashMap<String, CachedProjection>,
    ttl_ms: u64,
    max_entries: usize,
    max_size_bytes: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

struct CachedProjection {
    projection: super::events::Projection,
    inserted_at_ms: u64,
    size_bytes: usize,
    events: Vec<LensEvent>,
}

impl InMemoryProjectionCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            ttl_ms: 30_000,
            max_entries: 10_000,
            max_size_bytes: 100_000_000,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
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

impl Default for InMemoryProjectionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl super::traits::ProjectionCache for InMemoryProjectionCache {
    fn get_or_compute<L: Lens>(
        &self,
        lens: &L,
        events: &[LensEvent],
        input: &L::Input,
    ) -> super::events::Projection {
        let key = match lens.cache_key(input) {
            Some(k) => k,
            None => {
                let output_value = serde_json::to_value(lens.project(events, input))
                    .expect("Output serialization failed");
                return super::events::Projection {
                    lens_id: lens.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events
                        .last()
                        .map(|e| e.vector_clock.clone())
                        .unwrap_or_default(),
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
                self.hits.fetch_add(1, Ordering::Relaxed);
                let mut projection = cached.projection.clone();
                projection.was_cached = true;
                return projection;
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        let output_value =
            serde_json::to_value(lens.project(events, input)).expect("Output serialization failed"); // foxguard: ignore[rs/no-unwrap-in-lib]
        let projection = super::events::Projection {
            lens_id: lens.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events
                .last()
                .map(|e| e.vector_clock.clone())
                .unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes =
            std::mem::size_of::<super::events::Projection>() + projection.output.to_string().len();

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
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
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
                self.evictions.fetch_add(1, Ordering::Relaxed);
                self.cache.remove(&key);
            }
        }

        let mut total_size = self
            .cache
            .iter()
            .map(|e| e.value().size_bytes)
            .sum::<usize>();
        while total_size > self.max_size_bytes && !self.cache.is_empty() {
            if let Some(key) = self
                .cache
                .iter()
                .min_by_key(|e| e.value().inserted_at_ms)
                .map(|e| e.key().clone())
            {
                if let Some((_, removed)) = self.cache.remove(&key) {
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    total_size -= removed.size_bytes;
                }
            }
        }
    }
}

pub struct IncrementalProjectionCache {
    base_projections: DashMap<String, CachedProjection>,
    delta_events: DashMap<String, Vec<LensEvent>>,
    ttl_ms: u64,
    max_entries: usize,
    max_size_bytes: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl IncrementalProjectionCache {
    pub fn new() -> Self {
        Self {
            base_projections: DashMap::new(),
            delta_events: DashMap::new(),
            ttl_ms: 30_000,
            max_entries: 10_000,
            max_size_bytes: 100_000_000,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
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
        self.on_new_event_for_lenses(
            event,
            super::registry::LensRegistry::registered_lenses().as_slice(),
        )
    }

    pub fn on_new_event_for_lenses(
        &self,
        event: &LensEvent,
        lenses: &[&'static dyn super::traits::Projectable],
    ) {
        for lens in lenses {
            if event.aggregate_id.starts_with(lens.id().as_str())
                || lens.id().as_str().starts_with(&event.aggregate_id)
            {
                let key = event.aggregate_id.clone();
                self.delta_events
                    .entry(key)
                    .or_default()
                    .push(event.clone());
            }
        }
    }

    pub fn get_or_compute_projectable(
        &self,
        projectable: &dyn super::traits::Projectable,
        events: &[LensEvent],
        input: &serde_json::Value,
    ) -> super::events::Projection {
        let key = match projectable.cache_key_value(input) {
            Some(k) => k,
            None => {
                let output_value = projectable.project_value(events, input);
                return super::events::Projection {
                    lens_id: projectable.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events
                        .last()
                        .map(|e| e.vector_clock.clone())
                        .unwrap_or_default(),
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
            let delta = self
                .delta_events
                .get(&key)
                .map(|v| v.value().clone())
                .unwrap_or_default();
            let combined: Vec<_> = base.events.iter().chain(delta.iter()).cloned().collect();
            let output_value = projectable.project_value(&combined, input);
            let projection = super::events::Projection {
                lens_id: projectable.id().clone(),
                input_hash: compute_hash(&output_value),
                output: output_value,
                vector_clock: combined
                    .last()
                    .map(|e| e.vector_clock.clone())
                    .unwrap_or_default(),
                token_count: None,
                summary: None,
                projection_time_ms: 0,
                event_count: combined.len(),
                was_cached: true,
                last_sequence_number: combined.last().map(|e| e.sequence_number),
            };
            return projection;
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        let output_value = projectable.project_value(events, input);
        let projection = super::events::Projection {
            lens_id: projectable.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events
                .last()
                .map(|e| e.vector_clock.clone())
                .unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes =
            std::mem::size_of::<super::events::Projection>() + projection.output.to_string().len();

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

impl Default for IncrementalProjectionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl super::traits::ProjectionCache for IncrementalProjectionCache {
    fn get_or_compute<L: Lens>(
        &self,
        lens: &L,
        events: &[LensEvent],
        input: &L::Input,
    ) -> super::events::Projection {
        let key = match lens.cache_key(input) {
            Some(k) => k,
            None => {
                let output_value = serde_json::to_value(lens.project(events, input))
                    .expect("Output serialization failed");
                return super::events::Projection {
                    lens_id: lens.id().clone(),
                    input_hash: compute_hash(&output_value),
                    output: output_value,
                    vector_clock: events
                        .last()
                        .map(|e| e.vector_clock.clone())
                        .unwrap_or_default(),
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
            let delta = self
                .delta_events
                .get(&key)
                .map(|v| v.value().clone())
                .unwrap_or_default();
            let combined: Vec<_> = base.events.iter().chain(delta.iter()).cloned().collect();
            let output_value = serde_json::to_value(lens.project(&combined, input))
                .expect("Output serialization failed");
            let projection = super::events::Projection {
                lens_id: lens.id().clone(),
                input_hash: compute_hash(&output_value),
                output: output_value,
                vector_clock: combined
                    .last()
                    .map(|e| e.vector_clock.clone())
                    .unwrap_or_default(),
                token_count: None,
                summary: None,
                projection_time_ms: 0,
                event_count: combined.len(),
                was_cached: true,
                last_sequence_number: combined.last().map(|e| e.sequence_number),
            };
            return projection;
        }

        self.misses.fetch_add(1, Ordering::Relaxed);

        let output_value =
            serde_json::to_value(lens.project(events, input)).expect("Output serialization failed");
        let projection = super::events::Projection {
            lens_id: lens.id().clone(),
            input_hash: compute_hash(&output_value),
            output: output_value,
            vector_clock: events
                .last()
                .map(|e| e.vector_clock.clone())
                .unwrap_or_default(),
            token_count: None,
            summary: None,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        };

        let size_bytes =
            std::mem::size_of::<super::events::Projection>() + projection.output.to_string().len();

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
        self.base_projections
            .retain(|_, v| v.projection.lens_id != *lens_id);
        self.delta_events
            .retain(|k, _| !k.starts_with(lens_id.as_str()));
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
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            entries: self.base_projections.len(),
            total_size_bytes: self
                .base_projections
                .iter()
                .map(|e| e.value().size_bytes)
                .sum(),
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
                self.evictions.fetch_add(1, Ordering::Relaxed);
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
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    total_size -= removed.size_bytes;
                }
            }
        }
    }
}

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

fn now_ms() -> u64 {
    std::time::SystemTime::now() // foxguard: ignore[rs/no-unwrap-in-lib] — duration_since UNIX_EPOCH cannot fail on supported platforms
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as u64
}
