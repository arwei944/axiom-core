//! Axiom Store - Immutable event log, the single source of truth.

pub mod event;
pub mod memory;
pub mod snapshot;
pub mod store;
pub mod replay;
pub mod metrics;

pub use event::{Event, EventBuilder, EventMetadata};
pub use memory::MemoryStore;
pub use snapshot::{MemorySnapshotStore, Snapshot, SnapshotPolicy, SnapshotStore};
pub use store::{EventStore, StoreError};
pub use replay::{ReplayEngine, ReplayResult, ReplayableState};
pub use metrics::{MeteredStore, StoreHealth, StoreMetrics};
