use crate::id::LensId;
use thiserror::Error;

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
