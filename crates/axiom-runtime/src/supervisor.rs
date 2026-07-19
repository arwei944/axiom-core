//! Supervisor - per-cell crash recovery and circuit breaker.
//!
//! Each Cell runs under a Supervisor that:
//! 1. Catches panics via catch_unwind
//! 2. Restarts the Cell according to its SupervisionStrategy
//! 3. Implements circuit breaker to prevent error cascades
//! 4. Tracks consecutive failures and backoff timing

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub enum CellState {
    Running,
    Restarting { attempt: u32 },
    CircuitOpen { until: Instant },
    Stopped,
}

pub struct CircuitBreaker {
    failure_count: u32,
    failure_threshold: u32,
    reset_after: Duration,
    last_failure: Option<Instant>,
    state: CBState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CBState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_after_ms: u64) -> Self {
        Self {
            failure_count: 0,
            failure_threshold,
            reset_after: Duration::from_millis(reset_after_ms),
            last_failure: None,
            state: CBState::Closed,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CBState::Closed;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        if self.failure_count >= self.failure_threshold {
            self.state = CBState::Open;
        }
    }

    pub fn allow_call(&mut self) -> bool {
        match self.state {
            CBState::Closed => true,
            CBState::Open => {
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.reset_after {
                        self.state = CBState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CBState::HalfOpen => true,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == CBState::Open
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Snapshot for EventStore persistence (P0-5).
    pub fn snapshot(&self) -> CircuitSnapshot {
        CircuitSnapshot {
            failure_count: self.failure_count,
            failure_threshold: self.failure_threshold,
            reset_after_ms: self.reset_after.as_millis() as u64,
            open: self.state == CBState::Open,
        }
    }

    pub fn restore(snap: &CircuitSnapshot) -> Self {
        let mut cb = Self::new(snap.failure_threshold, snap.reset_after_ms);
        cb.failure_count = snap.failure_count;
        if snap.open {
            cb.state = CBState::Open;
            cb.last_failure = Some(Instant::now());
        }
        cb
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircuitSnapshot {
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub reset_after_ms: u64,
    pub open: bool,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(3, 30_000)
    }
}

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub base_ms: u64,
    pub cap_ms: u64,
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base_ms: 100,
            cap_ms: 30_000,
            multiplier: 2.0,
        }
    }
}

pub struct Supervisor {
    states: RwLock<HashMap<String, CellSupervision>>,
    backoff: BackoffConfig,
}

struct CellSupervision {
    state: CellState,
    strategy: axiom_kernel::cell::SupervisionStrategy,
    circuit_breaker: CircuitBreaker,
    restart_count: u64,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            backoff: BackoffConfig::default(),
        }
    }

    pub fn with_backoff(backoff: BackoffConfig) -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            backoff,
        }
    }

    pub fn set_backoff(&mut self, backoff: BackoffConfig) {
        self.backoff = backoff;
    }

    pub async fn register_cell(
        &self,
        cell_id: &str,
        strategy: axiom_kernel::cell::SupervisionStrategy,
    ) {
        // P0-5: default circuit policy applies to all cells; CircuitBreak strategy overrides.
        let default_policy = axiom_kernel::cell::DefaultCircuitPolicy::default();
        let (threshold, reset_after_ms) = match strategy {
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak {
                failure_threshold,
                reset_after_ms,
            } => (failure_threshold, reset_after_ms),
            _ => (default_policy.failure_threshold, default_policy.reset_after_ms),
        };

        self.states.write().await.insert(
            cell_id.to_string(),
            CellSupervision {
                state: CellState::Running,
                strategy,
                circuit_breaker: CircuitBreaker::new(threshold, reset_after_ms),
                restart_count: 0,
            },
        );
    }

    /// Persist all circuit snapshots as events (P0-5).
    pub async fn persist_circuits_to_store(
        &self,
        store: &dyn axiom_store::EventStore,
    ) -> Result<(), String> {
        let states = self.states.read().await;
        for (cell_id, sup) in states.iter() {
            let snap = sup.circuit_breaker.snapshot();
            let mut event = axiom_store::Event::new(
                &format!("circuit:{cell_id}"),
                "circuit.snapshot",
                serde_json::to_value(&snap).map_err(|e| e.to_string())?,
            );
            event.cell_id = cell_id.clone();
            // unique event id per persist
            event.event_id = format!(
                "circuit-{}-{}",
                cell_id,
                axiom_kernel::clock::global_clock().now_ns()
            );
            store
                .append(event)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Restore circuit state from EventStore after crash (P0-5).
    pub async fn restore_circuits_from_store(
        &self,
        store: &dyn axiom_store::EventStore,
    ) -> Result<usize, String> {
        let all = store.read_all().await.map_err(|e| e.to_string())?;
        let mut restored = 0usize;
        let mut latest: HashMap<String, CircuitSnapshot> = HashMap::new();
        for e in all {
            if e.event_type == "circuit.snapshot" {
                if let Ok(snap) = serde_json::from_value::<CircuitSnapshot>(e.payload) {
                    let cell = if !e.cell_id.is_empty() {
                        e.cell_id.clone()
                    } else {
                        e.aggregate_id
                            .strip_prefix("circuit:")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| e.aggregate_id.clone())
                    };
                    latest.insert(cell, snap);
                }
            }
        }
        let mut states = self.states.write().await;
        for (cell_id, snap) in latest {
            if let Some(sup) = states.get_mut(&cell_id) {
                sup.circuit_breaker = CircuitBreaker::restore(&snap);
                if snap.open {
                    sup.state = CellState::CircuitOpen {
                        until: Instant::now() + Duration::from_millis(snap.reset_after_ms),
                    };
                }
                restored += 1;
            } else {
                states.insert(
                    cell_id.clone(),
                    CellSupervision {
                        state: if snap.open {
                            CellState::CircuitOpen {
                                until: Instant::now() + Duration::from_millis(snap.reset_after_ms),
                            }
                        } else {
                            CellState::Running
                        },
                        strategy: axiom_kernel::cell::SupervisionStrategy::default(),
                        circuit_breaker: CircuitBreaker::restore(&snap),
                        restart_count: 0,
                    },
                );
                restored += 1;
            }
        }
        Ok(restored)
    }

    pub async fn before_handle(&self, cell_id: &str) -> bool {
        let mut states = self.states.write().await;
        if let Some(s) = states.get_mut(cell_id) {
            s.circuit_breaker.allow_call()
        } else {
            false
        }
    }

    pub async fn record_success(&self, cell_id: &str) {
        let mut states = self.states.write().await;
        if let Some(s) = states.get_mut(cell_id) {
            s.circuit_breaker.record_success();
            s.state = CellState::Running;
        }
    }

    pub async fn record_panic(&self, cell_id: &str) -> SupervisionDecision {
        let mut states = self.states.write().await;
        let Some(s) = states.get_mut(cell_id) else {
            return SupervisionDecision::Stop;
        };

        s.circuit_breaker.record_failure();
        s.restart_count += 1;

        if s.circuit_breaker.is_open() {
            let until = Instant::now()
                + match s.strategy {
                    axiom_kernel::cell::SupervisionStrategy::CircuitBreak {
                        reset_after_ms,
                        ..
                    } => Duration::from_millis(reset_after_ms),
                    _ => Duration::from_secs(30),
                };
            s.state = CellState::CircuitOpen { until };
            return SupervisionDecision::CircuitBreak { until };
        }

        match s.strategy {
            axiom_kernel::cell::SupervisionStrategy::Stop => {
                s.state = CellState::Stopped;
                SupervisionDecision::Stop
            }
            axiom_kernel::cell::SupervisionStrategy::Escalate => SupervisionDecision::Escalate,
            axiom_kernel::cell::SupervisionStrategy::Restart { max_retries } => {
                if s.restart_count > max_retries as u64 {
                    s.state = CellState::Stopped;
                    SupervisionDecision::Stop
                } else {
                    let attempt = s.restart_count as u32;
                    s.state = CellState::Restarting { attempt };
                    let cfg = self.backoff.clone();
                    SupervisionDecision::Restart {
                        backoff_ms: Self::backoff_ms_with(
                            attempt,
                            cfg.base_ms,
                            cfg.cap_ms,
                            cfg.multiplier,
                        ),
                    }
                }
            }
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak { .. } => {
                let attempt = s.restart_count as u32;
                s.state = CellState::Restarting { attempt };
                let cfg = self.backoff.clone();
                SupervisionDecision::Restart {
                    backoff_ms: Self::backoff_ms_with(
                        attempt,
                        cfg.base_ms,
                        cfg.cap_ms,
                        cfg.multiplier,
                    ),
                }
            }
        }
    }

    /// Full-jitter exponential backoff (P1-2) using instance config when available.
    pub fn backoff_ms(&self, attempt: u32) -> u64 {
        Self::backoff_ms_with(
            attempt,
            self.backoff.base_ms,
            self.backoff.cap_ms,
            self.backoff.multiplier,
        )
    }

    pub fn backoff_ms_with(attempt: u32, base_ms: u64, cap_ms: u64, multiplier: f64) -> u64 {
        let exp = (multiplier.powi(attempt.min(16) as i32)) * base_ms as f64;
        let high = exp.min(cap_ms as f64).max(base_ms as f64) as u64;
        // full jitter: uniform in [0, high]
        let salt = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(1) as u64)
            .saturating_add(attempt as u64 * 17);
        if high == 0 {
            return 0;
        }
        salt % (high + 1)
    }

    pub async fn restart_count(&self, cell_id: &str) -> u64 {
        self.states.read().await.get(cell_id).map(|s| s.restart_count).unwrap_or(0)
    }

    pub async fn is_circuit_open(&self, cell_id: &str) -> bool {
        self.states.read().await.get(cell_id).map(|s| s.circuit_breaker.is_open()).unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum SupervisionDecision {
    Restart { backoff_ms: u64 },
    Stop,
    Escalate,
    CircuitBreak { until: Instant },
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod jitter_tests {
    use super::*;

    #[test]
    fn full_jitter_disperses_under_concurrent_failures() {
        let mut values = std::collections::HashSet::new();
        for attempt in 0..100u32 {
            values.insert(Supervisor::backoff_ms_with(attempt % 5, 100, 5000, 2.0));
        }
        // Expect dispersion: more than one distinct delay
        assert!(values.len() > 5, "jitter not dispersing: {values:?}");
    }

    #[test]
    fn circuit_snapshot_roundtrip() {
        let mut cb = CircuitBreaker::new(3, 1000);
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        let snap = cb.snapshot();
        let restored = CircuitBreaker::restore(&snap);
        assert!(restored.is_open());
        assert_eq!(restored.failure_count(), 3);
    }

    #[test]
    fn backoff_config_override_is_consumed() {
        let s = Supervisor::with_backoff(BackoffConfig {
            base_ms: 50,
            cap_ms: 200,
            multiplier: 2.0,
        });
        for _ in 0..20 {
            let v = s.backoff_ms(3);
            assert!(v <= 200, "cap not applied: {v}");
        }
    }

    #[tokio::test]
    async fn circuit_persist_restore_after_crash() {
        let store = axiom_store::MemoryStore::new();
        let sup = Supervisor::new();
        sup.register_cell(
            "c1",
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak {
                failure_threshold: 2,
                reset_after_ms: 60_000,
            },
        )
        .await;
        // trip circuit
        let _ = sup.record_panic("c1").await;
        let _ = sup.record_panic("c1").await;
        assert!(sup.is_circuit_open("c1").await);
        sup.persist_circuits_to_store(&store).await.unwrap();

        // "crash": new supervisor restores from store
        let sup2 = Supervisor::new();
        let n = sup2.restore_circuits_from_store(&store).await.unwrap();
        assert!(n >= 1);
        assert!(sup2.is_circuit_open("c1").await);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let mut cb = CircuitBreaker::new(3, 60_000);
        assert!(cb.allow_call());
        cb.record_failure();
        cb.record_failure();
        assert!(cb.allow_call());
        cb.record_failure();
        assert!(!cb.allow_call(), "circuit should open after 3 failures");
    }

    #[test]
    fn test_circuit_breaker_recovers() {
        let mut cb = CircuitBreaker::new(2, 10);
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.allow_call());
        std::thread::sleep(Duration::from_millis(20));
        assert!(cb.allow_call(), "should half-open after reset window");
    }

    #[test]
    fn test_circuit_breaker_success_resets() {
        let mut cb = CircuitBreaker::new(3, 60_000);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert!(cb.allow_call());
        assert!(!cb.is_open());
    }

    #[tokio::test]
    async fn test_supervisor_restart_strategy() {
        let sup = Supervisor::new();
        sup.register_cell(
            "test-cell",
            axiom_kernel::cell::SupervisionStrategy::Restart { max_retries: 3 },
        )
        .await;
        let d1 = sup.record_panic("test-cell").await;
        assert!(matches!(d1, SupervisionDecision::Restart { .. }));
        let _ = sup.record_panic("test-cell").await;
        let _ = sup.record_panic("test-cell").await;
        let d4 = sup.record_panic("test-cell").await;
        assert!(matches!(d4, SupervisionDecision::Stop), "after max retries should stop");
    }

    #[tokio::test]
    async fn test_supervisor_circuit_break_strategy() {
        let sup = Supervisor::new();
        sup.register_cell(
            "test-cell",
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak {
                failure_threshold: 2,
                reset_after_ms: 0,
            },
        )
        .await;
        let _ = sup.record_panic("test-cell").await;
        let d2 = sup.record_panic("test-cell").await;
        assert!(matches!(d2, SupervisionDecision::CircuitBreak { .. }));
        assert!(sup.is_circuit_open("test-cell").await);
    }
}
