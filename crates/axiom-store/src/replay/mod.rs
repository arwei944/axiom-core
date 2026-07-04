//! Replay engine - rebuilds aggregate state from event log.

pub mod engine;
pub mod validation;
pub mod witness;

pub use engine::{ReplayEngine, ReplayResult, ReplayableState};
pub use validation::{validate_migration_chains_at_startup, StartupValidation, StateDiff};
pub use witness::{WitnessReplay, WitnessReplayResult};
