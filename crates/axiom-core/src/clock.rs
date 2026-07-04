//! Clock abstraction for deterministic time handling.
//!
//! Provides a `Clock` trait so time-dependent code can be tested with a
//! controllable clock instead of `std::time::SystemTime`.

use parking_lot::Mutex;
use std::sync::Arc;

/// Clock source for `now_ns()`.
///
/// Implementations provide the current time in nanoseconds. The default
/// source is [`SystemClock`]; tests can use [`MockClock`] to control time.
pub trait Clock: Send + Sync {
    /// Return the current time in nanoseconds since the UNIX epoch.
    fn now_ns(&self) -> u64;
}

/// System wall-clock source.
///
/// Delegates to `std::time::SystemTime` and is the default clock.
#[derive(Debug, Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

/// Mock clock for deterministic tests.
///
/// Starts at `0` and can be advanced or set manually.
#[derive(Debug, Clone)]
pub struct MockClock {
    now: Arc<Mutex<u64>>,
}

impl Default for MockClock {
    fn default() -> Self {
        Self {
            now: Arc::new(Mutex::new(0)),
        }
    }
}

impl MockClock {
    /// Create a new mock clock initialized to `0`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the clock by `delta_ns` nanoseconds.
    pub fn advance(&self, delta_ns: u64) {
        let mut now = self.now.lock();
        *now = now.saturating_add(delta_ns);
    }

    /// Set the clock to an absolute `timestamp_ns`.
    pub fn set(&self, timestamp_ns: u64) {
        *self.now.lock() = timestamp_ns;
    }

    /// Return the current mock time (for assertions).
    pub fn current(&self) -> u64 {
        *self.now.lock()
    }
}

impl Clock for MockClock {
    fn now_ns(&self) -> u64 {
        *self.now.lock()
    }
}

// === Global clock singleton ===

use std::sync::OnceLock;

static GLOBAL_CLOCK: OnceLock<parking_lot::Mutex<Arc<dyn Clock>>> = OnceLock::new();

fn global_clock_lock() -> &'static parking_lot::Mutex<Arc<dyn Clock>> {
    GLOBAL_CLOCK.get_or_init(|| Mutex::new(Arc::new(SystemClock)))
}

/// Return the global clock instance.
pub fn global_clock() -> Arc<dyn Clock> {
    global_clock_lock().lock().clone()
}

/// Replace the global clock instance. Intended for tests only.
pub fn set_global_clock(clock: Arc<dyn Clock>) {
    *global_clock_lock().lock() = clock;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_returns_current_time() {
        let clock = SystemClock;
        let t1 = clock.now_ns();
        assert!(t1 > 0);
    }

    #[test]
    fn mock_clock_starts_at_zero() {
        let clock = MockClock::new();
        assert_eq!(clock.now_ns(), 0);
    }

    #[test]
    fn mock_clock_advance_increments_time() {
        let clock = MockClock::new();
        clock.advance(100);
        assert_eq!(clock.now_ns(), 100);
        clock.advance(50);
        assert_eq!(clock.now_ns(), 150);
    }

    #[test]
    fn mock_clock_set_sets_absolute_time() {
        let clock = MockClock::new();
        clock.set(1_000_000);
        assert_eq!(clock.now_ns(), 1_000_000);
        clock.set(500);
        assert_eq!(clock.now_ns(), 500);
    }

    #[test]
    fn mock_clock_saturating_advance() {
        let clock = MockClock::new();
        clock.set(u64::MAX);
        clock.advance(1);
        assert_eq!(clock.now_ns(), u64::MAX);
    }
}
