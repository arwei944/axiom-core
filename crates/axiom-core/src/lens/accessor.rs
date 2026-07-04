use super::error::LensError;
use super::events::LensEvent;
use super::traits::Projectable;
use crate::id::LensId;
use serde::{Deserialize, Serialize};

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
    ) -> Result<super::events::Projection, LensError>
    where
        I: Serialize + Send + Sync + 'static,
        O: Deserialize<'static> + Send + Sync + 'static,
    {
        let input_value =
            serde_json::to_value(input).map_err(|e| LensError::Serialization(e.to_string()))?;

        let output_value = self.projectable.project_value(events, &input_value);

        let token_count = self.projectable.token_estimate_value(&output_value);
        let summary = self.projectable.summary_value(&output_value);

        let input_hash = compute_hash(&input_value);

        Ok(super::events::Projection {
            lens_id: self.projectable.id().clone(),
            input_hash,
            output: output_value,
            vector_clock: events
                .last()
                .map(|e| e.vector_clock.clone())
                .unwrap_or_default(),
            token_count,
            summary,
            projection_time_ms: 0,
            event_count: events.len(),
            was_cached: false,
            last_sequence_number: events.last().map(|e| e.sequence_number),
        })
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
