#[derive(Debug, Clone)]
pub struct SqliteStoreConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub migration_timeout_ms: u64,
}

impl Default for SqliteStoreConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:axiom_events.db".to_string(),
            max_connections: 5,
            migration_timeout_ms: 5000,
        }
    }
}
