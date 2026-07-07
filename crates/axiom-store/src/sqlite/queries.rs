use crate::event::Event;
use crate::sqlite::store::SqliteStore;
use crate::store::{BoxFuture, EventReceiver, EventStore, StoreError};
use sqlx::Row;

impl EventStore for SqliteStore {
    fn append<'a>(&'a self, event: Event) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let event_json = serde_json::to_string(&event.payload)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let vector_clock_json = serde_json::to_string(&event.vector_clock)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let layer_json = serde_json::to_string(&event.metadata.layer)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let outcome_json = serde_json::to_string(&event.metadata.outcome)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let witness_hash_bytes = event.metadata.witness_hash.as_ref().map(|h| h.hash.to_vec());
            let witness_hash_prev =
                event.metadata.witness_hash.as_ref().and_then(|h| h.prev_hash.map(|b| b.to_vec()));
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
            let signal_fingerprint =
                event.metadata.witness_hash.as_ref().map(|h| h.signal_fingerprint.to_vec());

            let seq = sqlx::query(
                r#"
                INSERT INTO events (
                    event_id, aggregate_id, cell_id, correlation_id, triggering_msg_id, vector_clock,
                    timestamp_ns, payload, event_type, schema_version, layer,
                    processing_time_ms, was_replayed, outcome, summary,
                    witness_hash_prev, witness_hash_before, witness_hash_after, witness_hash,
                    signal_fingerprint, payload_size_bytes
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&event.event_id)
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
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite append: {e}")))?;

            let sequence_number = seq.last_insert_rowid() as u64;
            let arc_event = std::sync::Arc::new(event.clone());
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
                let vector_clock_json = serde_json::to_string(&event.vector_clock)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let layer_json = serde_json::to_string(&event.metadata.layer)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let outcome_json = serde_json::to_string(&event.metadata.outcome)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                let witness_hash_bytes =
                    event.metadata.witness_hash.as_ref().map(|h| h.hash.to_vec());
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
                let signal_fingerprint =
                    event.metadata.witness_hash.as_ref().map(|h| h.signal_fingerprint.to_vec());

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
                .bind(event.triggering_msg_id.as_ref().map(|m| m.as_str()))
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
                let arc_event = std::sync::Arc::new(event.clone());
                let _ = self.sender.send(arc_event);
            }
            tx.commit().await.map_err(|e| StoreError::Storage(format!("sqlite tx commit: {e}")))?;
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
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
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StoreError::Storage(format!("sqlite read_by_time_range: {e}")))?;

            rows.iter().map(Self::row_to_event).collect()
        })
    }

    fn latest_sequence<'a>(&'a self) -> BoxFuture<'a, Result<u64, StoreError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT MAX(sequence_number) as max_seq FROM events")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| StoreError::Storage(format!("sqlite latest_sequence: {e}")))?;
            Ok(row.get::<Option<i64>, _>("max_seq").unwrap_or(0) as u64)
        })
    }

    fn subscribe(&self) -> EventReceiver {
        self.sender.subscribe()
    }
}
