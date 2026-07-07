use axiom_kernel::axiom::{DynLens, Lens, Projection, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[axiom_macros::lens(
    id = "test-lens",
    depends_on = [],
    cache = true,
    version = "1.0.0"
)]
struct TestLensInput {
    value: String,
}

impl Lens for TestLensInput {
    fn id(&self) -> &'static str {
        "test-lens"
    }

    fn project(&self, _state: &State) -> ::axiom_kernel::KernelResult<Projection> {
        Ok(Projection::new(serde_json::to_vec(self).unwrap()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[axiom_macros::lens(
    id = "aggregate-lens",
    depends_on = [],
    cache = true,
    version = "1.0.0"
)]
struct AggregateLensInput {
    aggregate_id: String,
}

impl Lens for AggregateLensInput {
    fn id(&self) -> &'static str {
        "aggregate-lens"
    }

    fn project(&self, _state: &State) -> ::axiom_kernel::KernelResult<Projection> {
        Ok(Projection::new(serde_json::to_vec(self).unwrap()))
    }
}

#[test]
fn lens_macro_generates_lens_impl() {
    let lens = TestLensInput {
        value: "test".to_string(),
    };

    let id = Lens::id(&lens);
    assert_eq!(id, "test-lens");
}

#[test]
fn lens_macro_generates_dyn_lens_impl() {
    let lens = TestLensInput {
        value: "test".to_string(),
    };

    let dyn_lens: &dyn DynLens = &lens;
    assert_eq!(dyn_lens.id(), "test-lens");
}

#[test]
fn lens_macro_project_works() {
    let lens = TestLensInput {
        value: "test".to_string(),
    };
    let state = State::empty();

    let result = Lens::project(&lens, &state);
    assert!(result.is_ok());
}

#[test]
fn lens_macro_aggregate_project_works() {
    let lens = AggregateLensInput {
        aggregate_id: "agg-1".to_string(),
    };
    let state = State::empty();

    let result = Lens::project(&lens, &state);
    assert!(result.is_ok());
}
