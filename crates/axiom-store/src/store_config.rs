//! Store configuration and factory for selecting backends at runtime.

use crate::file_store::{FileStore, FileStoreConfig};
use crate::memory::MemoryStore;
#[cfg(feature = "sqlite")]
use crate::sqlite::{SqliteStore, SqliteStoreConfig};
use crate::store::EventStore;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum StoreConfig {
    Memory,
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteStoreConfig),
    File(FileStoreConfig),
}

impl Default for StoreConfig {
    fn default() -> Self {
        StoreConfig::Memory
    }
}

pub enum StoreFactory {
    Memory(Arc<dyn EventStore>),
    #[cfg(feature = "sqlite")]
    Sqlite(Arc<SqliteStore>),
    File(Arc<FileStore>),
}

impl StoreFactory {
    pub async fn from_config(config: StoreConfig) -> Result<Self, crate::StoreError> {
        match config {
            StoreConfig::Memory => Ok(StoreFactory::Memory(Arc::new(MemoryStore::new()))),
            #[cfg(feature = "sqlite")]
            StoreConfig::Sqlite(cfg) => {
                let store = SqliteStore::connect(cfg).await?;
                Ok(StoreFactory::Sqlite(Arc::new(store)))
            }
            StoreConfig::File(cfg) => {
                let store = FileStore::connect(cfg).await?;
                Ok(StoreFactory::File(Arc::new(store)))
            }
        }
    }

    pub fn event_store(&self) -> Arc<dyn EventStore> {
        match self {
            StoreFactory::Memory(store) => store.clone(),
            #[cfg(feature = "sqlite")]
            StoreFactory::Sqlite(store) => store.clone(),
            StoreFactory::File(store) => store.clone(),
        }
    }
}
