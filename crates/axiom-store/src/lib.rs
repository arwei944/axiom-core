//! Axiom Store - Immutable event log, the single source of truth.

pub mod event;
pub mod memory;
pub mod metrics;
pub mod replay;
pub mod snapshot;
pub mod store;
pub mod store_config;

pub mod file_store;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use event::{Event, EventBuilder, EventMetadata, EventOutcome, WitnessHashData};
pub use memory::MemoryStore;
pub use metrics::{MeteredStore, StoreHealth, StoreMetrics};
pub use replay::{ReplayEngine, ReplayResult, ReplayableState};
pub use snapshot::{
    FileSnapshotStore, FileSnapshotStoreConfig, MemorySnapshotStore, Snapshot, SnapshotPolicy,
    SnapshotStore,
};
pub use store::{verify_witness_chain, EventStore, StoreError};
pub use store_config::{StoreConfig, StoreFactory};

#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteStore, SqliteStoreConfig};

pub use file_store::{FileStore, FileStoreConfig};

use axiom_kernel::KernelError;

impl From<StoreError> for KernelError {
    fn from(e: StoreError) -> Self {
        KernelError::Store(e.to_string())
    }
}
