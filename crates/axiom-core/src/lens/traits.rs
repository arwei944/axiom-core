use super::cache::CacheMetrics;
use super::events::{LensEvent, Projection};
use crate::id::LensId;
use serde::{Deserialize, Serialize};

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

    fn cache_key(&self, _input: &Self::Input) -> Option<String> {
        None
    }

    fn depends_on(&self) -> &[LensId] {
        &[]
    }

    fn token_estimate(&self, _output: &Self::Output) -> Option<usize> {
        None
    }

    fn summary(&self, _output: &Self::Output) -> Option<String> {
        None
    }
}

pub trait Projectable: Send + Sync + 'static {
    fn id(&self) -> &LensId;

    fn project_value(&self, events: &[LensEvent], input: &serde_json::Value) -> serde_json::Value;

    fn cache_key_value(&self, _input: &serde_json::Value) -> Option<String> {
        None
    }

    fn depends_on(&self) -> &[LensId] {
        &[]
    }

    fn token_estimate_value(&self, _output: &serde_json::Value) -> Option<usize> {
        None
    }

    fn summary_value(&self, _output: &serde_json::Value) -> Option<String> {
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
        let typed_input: L::Input =
            serde_json::from_value(input.clone()).expect("Input deserialization failed"); // foxguard: ignore[rs/no-unwrap-in-lib]
        let typed_output = Lens::project(self, events, &typed_input);
        serde_json::to_value(typed_output).expect("Output serialization failed")
        // foxguard: ignore[rs/no-unwrap-in-lib]
    }

    fn cache_key_value(&self, input: &serde_json::Value) -> Option<String> {
        let typed_input: L::Input =
            serde_json::from_value(input.clone()).expect("Input deserialization failed");
        Lens::cache_key(self, &typed_input)
    }

    fn depends_on(&self) -> &[LensId] {
        Lens::depends_on(self)
    }

    fn token_estimate_value(&self, output: &serde_json::Value) -> Option<usize> {
        let typed_output: L::Output =
            serde_json::from_value(output.clone()).expect("Output deserialization failed");
        Lens::token_estimate(self, &typed_output)
    }

    fn summary_value(&self, output: &serde_json::Value) -> Option<String> {
        let typed_output: L::Output =
            serde_json::from_value(output.clone()).expect("Output deserialization failed"); // foxguard: ignore[rs/no-unwrap-in-lib]
        Lens::summary(self, &typed_output)
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
