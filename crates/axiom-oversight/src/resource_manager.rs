//! ResourceManager - token bucket rate limiting and resource budgets.

use axiom_core::id::CellId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub tokens_available: u64,
    pub tokens_capacity: u64,
    pub refill_rate_per_sec: f64,
    pub rejected_acquires: u64,
    pub successful_acquires: u64,
    pub active_concurrency: u64,
    pub max_concurrency: u64,
}

pub struct TokenBucket {
    capacity: u64,
    tokens: AtomicU64,
    refill_rate_per_sec: f64,
    last_refill: Mutex<Instant>,
    rejected: AtomicU64,
    accepted: AtomicU64,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate_per_sec: f64) -> Self {
        Self {
            capacity,
            tokens: AtomicU64::new(capacity),
            refill_rate_per_sec,
            last_refill: Mutex::new(Instant::now()),
            rejected: AtomicU64::new(0),
            accepted: AtomicU64::new(0),
        }
    }

    pub fn try_acquire(&self, n: u64) -> bool {
        self.refill();
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current < n {
                self.rejected.fetch_add(1, Ordering::Relaxed);
                return false;
            }
            match self.tokens.compare_exchange_weak(
                current,
                current - n,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.accepted.fetch_add(1, Ordering::Relaxed);
                    return true;
                }
                Err(v) => current = v,
            }
        }
    }

    pub fn release(&self, n: u64) {
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            let new = (current + n).min(self.capacity);
            match self.tokens.compare_exchange_weak(
                current,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(v) => current = v,
            }
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        if elapsed < 0.001 {
            return;
        }
        let add = (elapsed * self.refill_rate_per_sec) as u64;
        if add == 0 {
            return;
        }
        let mut cur = self.tokens.load(Ordering::Relaxed);
        loop {
            let new = (cur + add).min(self.capacity);
            match self.tokens.compare_exchange_weak(
                cur,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => cur = v,
            }
        }
        *last = now;
    }

    pub fn available(&self) -> u64 {
        self.refill();
        self.tokens.load(Ordering::Relaxed)
    }

    pub fn stats(&self) -> ResourceStats {
        ResourceStats {
            tokens_available: self.available(),
            tokens_capacity: self.capacity,
            refill_rate_per_sec: self.refill_rate_per_sec,
            rejected_acquires: self.rejected.load(Ordering::Relaxed),
            successful_acquires: self.accepted.load(Ordering::Relaxed),
            active_concurrency: 0,
            max_concurrency: 0,
        }
    }
}

pub struct ConcurrencyLimiter {
    active: AtomicU64,
    max: u64,
    rejected: AtomicU64,
}

impl ConcurrencyLimiter {
    pub fn new(max: u64) -> Self {
        Self {
            active: AtomicU64::new(0),
            max,
            rejected: AtomicU64::new(0),
        }
    }
    pub fn try_enter(&self) -> bool {
        let cur = self.active.fetch_add(1, Ordering::Relaxed);
        if cur >= self.max {
            self.active.fetch_sub(1, Ordering::Relaxed);
            self.rejected.fetch_add(1, Ordering::Relaxed);
            false
        } else {
            true
        }
    }
    pub fn exit(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }
    pub fn active(&self) -> u64 {
        self.active.load(Ordering::Relaxed)
    }
    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }
}

pub struct ResourceManagerCell {
    id: CellId,
    tokens: Arc<TokenBucket>,
    concurrency: Arc<ConcurrencyLimiter>,
    per_cell: Arc<Mutex<HashMap<String, Arc<TokenBucket>>>>,
}

impl ResourceManagerCell {
    pub fn new(token_capacity: u64, refill_per_sec: f64, max_concurrency: u64) -> Self {
        Self {
            id: CellId::new("oversight:resource-manager"),
            tokens: Arc::new(TokenBucket::new(token_capacity, refill_per_sec)),
            concurrency: Arc::new(ConcurrencyLimiter::new(max_concurrency)),
            per_cell: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn global_tokens(&self) -> Arc<TokenBucket> {
        self.tokens.clone()
    }
    pub fn concurrency(&self) -> Arc<ConcurrencyLimiter> {
        self.concurrency.clone()
    }

    pub fn cell_tokens(&self, cell_id: &str) -> Arc<TokenBucket> {
        let mut map = self.per_cell.lock().unwrap();
        map.entry(cell_id.to_string())
            .or_insert_with(|| Arc::new(TokenBucket::new(1000, 100.0)))
            .clone()
    }

    pub fn stats(&self) -> ResourceStats {
        let mut s = self.tokens.stats();
        s.active_concurrency = self.concurrency.active();
        s.max_concurrency = self.concurrency.max;
        s.rejected_acquires += self.concurrency.rejected();
        s
    }
}

impl Default for ResourceManagerCell {
    fn default() -> Self {
        Self::new(10_000, 1000.0, 256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_acquire_release() {
        let b = TokenBucket::new(10, 100.0);
        assert!(b.try_acquire(5));
        assert_eq!(b.available(), 5);
        assert!(b.try_acquire(5));
        assert!(!b.try_acquire(1));
        b.release(10);
        assert_eq!(b.available(), 10);
    }

    #[test]
    fn test_token_bucket_refills() {
        let b = TokenBucket::new(10, 1000.0);
        assert!(b.try_acquire(10));
        assert!(!b.try_acquire(1));
        std::thread::sleep(Duration::from_millis(20));
        assert!(b.available() >= 1);
        assert!(b.try_acquire(1));
    }

    #[test]
    fn test_concurrency_limiter() {
        let c = ConcurrencyLimiter::new(2);
        assert!(c.try_enter());
        assert!(c.try_enter());
        assert!(!c.try_enter());
        assert_eq!(c.active(), 2);
        c.exit();
        assert_eq!(c.active(), 1);
        assert!(c.try_enter());
    }
}
