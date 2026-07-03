//! SQLite-backed event store implementation.

use crate::event::Event;
use crate::store::{BoxFuture, EventReceiver, EventStore, StoreError};
use axiom_core::id::CorrelationId;
use serde_json::Value;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;

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

pub struct SqliteStore {
    pool: SqlitePool,
    sender: broadcast::Sender<Arc<Event>>,
}

impl SqliteStore {
    pub async fn connect(config: SqliteStoreConfig) -> Result<Self, StoreError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite connect: {e}")))?;

        let store = Self {
            pool,
            sender: broadcast::channel(1024).0,
        };
        store.run_migrations().await?;
        Ok(store)
    }

    pub async fn connect_with_pool(pool: SqlitePool) -> Result<Self, StoreError> {
        let sender = broadcast::channel(1024).0;
        let store = Self { pool, sender };
        store.run_migrations().await?;
        Ok(store)
    }

    async fn run_migrations(&self) -> Result<(), StoreError> {
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
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::Storage(format!("sqlite migration: {e}")))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_id)",
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::Storage(format!("sqlite index aggregate_id: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ns)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index timestamp_ns: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_correlation ON events(correlation_id)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index correlation_id: {e}")))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_cell_id ON events(cell_id)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite index cell_id: {e}")))?;

        Ok(())
    }

    fn row_to_event(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<Event, StoreError> {
        use sqlx::Row;
        let witness_hash: Option<Vec<u8>> = row.get("witness_hash");
        let witness_hash = witness_hash.and_then(|b| {
            let mut arr = [0u8; 32];
            if b.len() == 32 {
                arr.copy_from_slice(&b);
                Some(crate::store::WitnessHashData {
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
            sequence_number: row.get("sequence_number"),
            event_id: row.get("event_id"),
            aggregate_id: row.get("aggregate_id"),
            cell_id: row.get("cell_id"),
            correlation_id: CorrelationId::new(row.get::<&str, _>("correlation_id")),
            triggering_msg_id: row
                .get::<Option<&str>, _>("triggering_msg_id")
                .map(|s| axiom_core::id::MsgId::new(s)),
            vector_clock: serde_json::from_str(row.get::<&str, _>("vector_clock"))
                .unwrap_or_default(),
            timestamp_ns: row.get("timestamp_ns"),
            payload,
            event_type: row.get("event_type"),
            schema_version: axiom_core::version::SchemaVersion::new(
                row.get::<i32, _>("schema_version") as u32,
            ),
            metadata: crate::store::EventMetadata {
                layer: serde_json::from_str(row.get::<&str, _>("layer")).unwrap_or_default(),
                processing_time_ms: row.get("processing_time_ms"),
                was_replayed: row.get::<i32, _>("was_replayed") != 0,
                outcome: serde_json::from_str(row.get::<&str, _>("outcome")).unwrap_or_default(),
                summary: row.get("summary"),
                witness_hash,
                payload_size_bytes: row.get("payload_size_bytes"),
            },
        })
    }
}

impl EventStore for SqliteStore {
    fn append<'a>(&'a self, event: Event) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let event_json = serde_json::to_string(&event.payload)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let vector_clock_json =
                serde_json::to_string(&event.vector_clock)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let layer_json =
                serde_json::to_string(&event.metadata.layer)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let outcome_json =
                serde_json::to_string(&event.metadata.outcome)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let witness_hash_bytes = event
                .metadata
                .witness_hash
                .as_ref()
                .map(|h| h.hash.to_vec());
            let witness_hash_prev = event
                .metadata
                .witness_hash
                .as_ref()
                .and_then(|h| h.prev_hash.map(|b| b.to_vec()));
            let witness_hash_before = event
                .metadata
                .witness_hash
                .as_ref()
                .and_then(|h| h.state_before_hash.map(|b| b.to_vec()));
            let witness_hash_after = event
                .metadata
                .witness_hash
                .as_ref()
                .and_then(|h| h.state_after_hash.map(|b| b.to_vec()));
            let signal_fingerprint = event
                .metadata
                .witness_hash
                .as_ref()
                .map(|h| h.signal_fingerprint.to_vec());

            let seq = sqlx::query(
                r#"
                INSERT INTO events (
                    aggregate_id, cell_id, correlation_id, triggering_msg_id, vector_clock,
                    timestamp_ns, payload, event_type, schema_version, layer,
                    processing_time_ms, was_replayed, outcome, summary,
                    witness_hash_prev, witness_hash_before, witness_hash_after, witness_hash,
                    signal_fingerprint, payload_size_bytes
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&event.aggregate_id)
            .bind(&event.cell_id)
            .bind(event.correlation_id.as_str())
            .bind(
                event
                    .triggering_msg_id
                    .as_ref()
                    .map(|m| m.as_str()),
            )
            .bind(&vector_clock_json)
            .bind(event.timestamp_ns as i64)
            .bind(&event_json)
            .bind(&event.event_type)
            .bind(event.schema_version.0 as i32)
            .bind(&layer_json)
            .bind(event.metadata.processing_time_ms as i64)
            .bind(if event.metadata.was_replayed { 1 } else { 0 })
            .bind(&outcome_json)
            .bind(&event.metadata.summary)
            .bind(witness_hash_prev)
            .bind(witness_hash_before)
            .bind(witness_hash_after)
            .bind(witness_hash_bytes)
            .bind(signal_fingerprint)
            .bind(event.metadata.payload_size_bytes as i64)
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite append: {e}")))?;

            let sequence_number = seq.last_insert_rowid() as u64;
            let arc_event = Arc::new(event.clone());
            let _ = self.sender.send(arc_event);
            Ok(sequence_number)
        })
    }

    fn append_batch<'a>(
        &'a self,
        events: Vec<Event>,
    ) -> BoxFuture<'a, Result<Vec<u64>, StoreError>> {
        Box::pin(async move {
            let mut seqs = Vec::with_capacity(events.len());
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| StoreError::Storage(format!("sqlite tx begin: {e}")))?;
            for event in events {
                let event_json = serde_json::to_string(&event.payload)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let vector_clock_json =
                    serde_json::to_string(&event.vector_clock)
                        .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let layer_json =
                    serde_json::to_string(&event.metadata.layer)
                        .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let outcome_json =
                    serde_json::to_string(&event.metadata.outcome)
                        .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let witness_hash_bytes = event
                    .metadata
                    .witness_hash
                    .as_ref()
                    .map(|h| h.hash.to_vec());
                let witness_hash_prev = event
                    .metadata
                    .witness_hash
                    .as_ref()
                    .and_then(|h| h.prev_hash.map(|b| b.to_vec()));
                let witness_hash_before = event
                    .metadata
                    .witness_hash
                    .as_ref()
                    .and_then(|h| h.state_before_hash.map(|b| b.to_vec()));
                let witness_hash_after = event
                    .metadata
                    .witness_hash
                    .as_ref()
                    .and_then(|h| h.state_after_hash.map(|b| b.to_vec()));
                let signal_fingerprint = event
                    .metadata
                    .witness_hash
                    .as_ref()
                    .map(|h| h.signal_fingerprint.to_vec());

                let seq = sqlx::query(
                    r#"
                    INSERT INTO events (
                        aggregate_id, cell_id, correlation_id, triggering_msg_id, vector_clock,
                        timestamp_ns, payload, event_type, schema_version, layer,
                        processing_time_ms, was_replayed, outcome, summary,
                        witness_hash_prev, witness_hash_before, witness_hash_after, witness_hash,
                        signal_fingerprint, payload_size_bytes
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&event.aggregate_id)
                .bind(&event.cell_id)
                .bind(event.correlation_id.as_str())
                .bind(
                    event
                        .triggering_msg_id
                        .as_ref()
                        .map(|m| m.as_str()),
                )
                .bind(&vector_clock_json)
                .bind(event.timestamp_ns as i64)
                .bind(&event_json)
                .bind(&event.event_type)
                .bind(event.schema_version.0 as i32)
                .bind(&layer_json)
                .bind(event.metadata.processing_time_ms as i64)
                .bind(if event.metadata.was_replayed { 1 } else { 0 })
                .bind(&outcome_json)
                .bind(&event.metadata.summary)
                .bind(witness_hash_prev)
                .bind(witness_hash_before)
                .bind(witness_hash_after)
                .bind(witness_hash_bytes)
                .bind(signal_fingerprint)
                .bind(event.metadata.payload_size_bytes as i64)
                .execute(&mut *tx)
                .await
                .map_err(|e| StoreError::Storage(format!("sqlite append_batch: {e}")))?;

                let sequence_number = seq.last_insert_rowid() as u64;
                seqs.push(sequence_number);
                let arc_event = Arc::new(event.clone());
                let _ = self.sender.send(arc_event);
            }
            tx.commit()
                .await
                .map_err(|e| StoreError::Storage(format!("sqlite tx commit: {e}")))?;
            Ok(seqs)
        })
    }

    fn read<'a>(&'a self, aggregate_id: &'a str) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE aggregate_id = ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(aggregate_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_all<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                ORDER BY sequence_number ASC
                "#,
            )
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_all: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_after<'a>(&'a self, after_ns: u64) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE timestamp_ns > ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(after_ns as i64)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_after: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_after_sequence<'a>(
        &'a self,
        seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE sequence_number > ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(seq as i64)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_after_sequence: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_range<'a>(
        &'a self,
        aggregate_id: &'a str,
        from_seq: u64,
        to_seq: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE aggregate_id = ? AND sequence_number >= ? AND sequence_number <= ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(aggregate_id)
            .bind(from_seq as i64)
            .bind(to_seq as i64)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_range: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_by_correlation<'a>(
        &'a self,
        correlation_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE correlation_id = ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(correlation_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_by_correlation: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_by_cell_id<'a>(
        &'a self,
        cell_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE cell_id = ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(cell_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_by_cell_id: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn read_by_time_range<'a>(
        &'a self,
        start_ns: u64,
        end_ns: u64,
    ) -> BoxFuture<'a, Result<Vec<Event>, StoreError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT sequence_number, event_id, aggregate_id, cell_id, correlation_id,
                       triggering_msg_id, vector_clock, timestamp_ns, payload, event_type,
                       schema_version, layer, processing_time_ms, was_replayed, outcome,
                       summary, witness_hash_prev, witness_hash_before, witness_hash_after,
                       witness_hash, signal_fingerprint, payload_size_bytes
                FROM events
                WHERE timestamp_ns >= ? AND timestamp_ns <= ?
                ORDER BY sequence_number ASC
                "#,
            )
            .bind(start_ns as i64)
            .bind(end_ns as i64)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_by_time_range: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT MAX(sequence_number) as max_seq FROM events")
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| StoreError::Storage(format!("sqlite latest_sequence: {e}")))?;
            Ok(row.get::<Option<i64>, _>("max_seq").unwrap_or(0) as u64)
        })
    }

    fn subscribe(&self) -> EventReceiver {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use std::sync::OnceLock;

    static DB_PATH: OnceLock<String> = OnceLock::new();

    fn db_path() -> String {
        DB_PATH
            .get_or_init(|| format!("file:test_sqlite_{}.db?mode=memory", uuid::Uuid::new_v4()))
            .clone()
    }

    #[tokio::test]
    async fn test_sqlite_roundtrip() {
        let pool = SqlitePool::connect(&db_path())
            .await
            .expect("sqlite pool");
        let store = SqliteStore::connect_with_pool(pool)
            .await
            .expect("sqlite store");
        let e = EventBuilder::new("a1", "evt1", serde_json::json!({}))
            .cell_id("c1")
            .build();
        let seq = store.append(e).await.unwrap();
        assert_eq!(seq, 1);
        let evts = store.read("a1").await.unwrap();
        assert_eq!(evts.len(), 1);
    }
}
