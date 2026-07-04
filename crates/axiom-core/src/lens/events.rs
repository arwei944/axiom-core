use crate::id::LensId;
use crate::signal::VectorClock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensEvent {
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub vector_clock: VectorClock,
    pub timestamp_ns: u64,
    pub sequence_number: u64,
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
        serde_json::from_value(self.output.clone()).map_err(|_| ProjectionDowncastError {
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
