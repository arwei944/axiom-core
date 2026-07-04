//! Append-only file-based event store implementation.

pub mod config;
pub mod store;

pub use config::FileStoreConfig;
pub use store::FileStore;
