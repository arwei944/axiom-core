use crate::event::Event;
use crate::store::StoreError;
use crate::SqliteStoreConfig;
use crate::WitnessHashData;
use axiom_kernel::id::CorrelationId;
use axiom_kernel::layer::RuntimeTier;
use serde_json::Value;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct SqliteStore {
    pub(crate) pool: SqlitePool,
    pub(crate) sender: broadcast::Sender<Arc<Event>>,
}

impl SqliteStore {
    pub async fn connect(config: SqliteStoreConfig) -> Result<Self, StoreError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite connect: {e}")))?;

        let store = Self { pool, sender: broadcast::channel(1024).0 };
        store.run_migrations().await?;
        Ok(store)
    }

    pub async fn connect_with_pool(pool: SqlitePool) -> Result<Self, StoreError> {
        let sender = broadcast::channel(1024).0;
        let store = Self { pool, sender };
        store.run_migrations().await?;
        Ok(store)
    }

    pub async fn run_migrations(&self) -> Result<(), StoreError> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite wal mode: {e}")))?;

        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite synchronous mode: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                sequence_number INTEGER PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                aggregate_id TEXT NOT NULL,
                cell_id TEXT NOT NULL,
                correlation_id TEXT NOT NULL,
                triggering_msg_id TEXT,
                vector_clock TEXT NOT NULL,
                timestamp_ns INTEGER NOT NULL,
                payload TEXT NOT NULL,
                event_type TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                layer TEXT NOT NULL,
                processing_time_ms INTEGER NOT NULL,
                was_replayed INTEGER NOT NULL,
                outcome TEXT NOT NULL,
                summary TEXT NOT NULL,
                witness_hash_prev TEXT,
                witness_hash_before TEXT,
                witness_hash_after TEXT,
                witness_hash TEXT,
                signal_fingerprint TEXT,
                payload_size_bytes INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Storage(format!("sqlite migration: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index aggregate_id: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ns)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index timestamp_ns: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_correlation ON events(correlation_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index correlation_id: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_cell_id ON events(cell_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index cell_id: {e}")))?;

        Ok(())
    }

    pub fn row_to_event(row: &sqlx::sqlite::SqliteRow) -> Result<Event, StoreError> {
        let witness_hash: Option<Vec<u8>> = row.get("witness_hash");
        let witness_hash = witness_hash.and_then(|b| {
            let mut arr = [0u8; 32];
            if b.len() == 32 {
                arr.copy_from_slice(&b);
                Some(WitnessHashData {
                    prev_hash: None,
                    state_before_hash: None,
                    state_after_hash: None,
                    hash: arr,
                    signal_fingerprint: [0u8; 32],
                })
            } else {
                None
            }
        });

        let payload: Value = serde_json::from_str(row.get::<&str, _>("payload"))
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        Ok(Event {
            sequence_number: row.get::<i64, _>("sequence_number") as u64,
            event_id: row.get("event_id"),
            aggregate_id: row.get("aggregate_id"),
            cell_id: row.get("cell_id"),
            correlation_id: CorrelationId::new(row.get::<&str, _>("correlation_id")),
            triggering_msg_id: row
                .get::<Option<&str>, _>("triggering_msg_id")
                .map(axiom_kernel::id::MsgId::new),
            vector_clock: serde_json::from_str(row.get::<&str, _>("vector_clock"))
                .unwrap_or_default(),
            timestamp_ns: row.get::<i64, _>("timestamp_ns") as u64,
            payload,
            event_type: row.get("event_type"),
            schema_version: axiom_kernel::version::SchemaVersion::new(
                row.get::<i32, _>("schema_version") as u16,
            ),
            metadata: crate::EventMetadata {
                layer: serde_json::from_str(row.get::<&str, _>("layer")).unwrap_or(RuntimeTier::Exec),
                processing_time_ms: row.get::<i64, _>("processing_time_ms") as u64,
                was_replayed: row.get::<i32, _>("was_replayed") != 0,
                outcome: serde_json::from_str(row.get::<&str, _>("outcome")).unwrap_or_default(),
                summary: row.get("summary"),
                witness_hash,
                payload_size_bytes: row.get::<i64, _>("payload_size_bytes") as usize,
            },
        })
    }
}
