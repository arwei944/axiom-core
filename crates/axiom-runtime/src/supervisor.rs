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
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(3, 30_000)
    }
}

pub struct Supervisor {
    states: RwLock<HashMap<String, CellSupervision>>,
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
        }
    }

    pub async fn register_cell(
        &self,
        cell_id: &str,
        strategy: axiom_kernel::cell::SupervisionStrategy,
    ) {
        let (threshold, reset_after_ms) = match strategy {
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak {
                failure_threshold,
                reset_after_ms,
            } => (failure_threshold, reset_after_ms),
            axiom_kernel::cell::SupervisionStrategy::Restart { .. } => (u32::MAX, u64::MAX),
            axiom_kernel::cell::SupervisionStrategy::Stop => (u32::MAX, u64::MAX),
            axiom_kernel::cell::SupervisionStrategy::Escalate => (u32::MAX, u64::MAX),
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
                        reset_after_ms, ..
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
                    SupervisionDecision::Restart {
                        backoff_ms: Self::backoff_ms(attempt),
                    }
                }
            }
            axiom_kernel::cell::SupervisionStrategy::CircuitBreak { .. } => {
                let attempt = s.restart_count as u32;
                s.state = CellState::Restarting { attempt };
                SupervisionDecision::Restart {
                    backoff_ms: Self::backoff_ms(attempt),
                }
            }
        }
    }

    fn backoff_ms(attempt: u32) -> u64 {
        let base = 100u64;
        let backoff = base * (1u64 << attempt.saturating_sub(1).min(9));
        backoff.min(30_000)
    }

    pub async fn restart_count(&self, cell_id: &str) -> u64 {
        self.states
            .read()
            .await
            .get(cell_id)
            .map(|s| s.restart_count)
            .unwrap_or(0)
    }

    pub async fn is_circuit_open(&self, cell_id: &str) -> bool {
        self.states
            .read()
            .await
            .get(cell_id)
            .map(|s| s.circuit_breaker.is_open())
            .unwrap_or(false)
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
        assert!(
            matches!(d4, SupervisionDecision::Stop),
            "after max retries should stop"
        );
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
