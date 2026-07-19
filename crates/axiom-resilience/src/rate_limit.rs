//! Token-bucket style rate limiter decorating a Port.

use axiom_isa::{IsaError, IsaResult, Port};
use axiom_kernel::clock::global_clock;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max permits available at full charge.
    pub capacity: u32,
    /// Permits restored per second (integer).
    pub refill_per_sec: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            capacity: 10,
            refill_per_sec: 10,
        }
    }
}

/// Rejects Port calls when the bucket is empty.
pub struct RateLimit<P> {
    name: String,
    inner: P,
    config: RateLimitConfig,
    tokens: f64,
    last_refill_ns: u64,
    pub rejected: u32,
}

impl<P> RateLimit<P> {
    pub fn new(inner: P, config: RateLimitConfig) -> Self {
        let cap = config.capacity as f64;
        Self {
            name: "rate_limit".into(),
            inner,
            config,
            tokens: cap,
            last_refill_ns: global_clock().now_ns(),
            rejected: 0,
        }
    }

    pub fn wrap(inner: P) -> Self {
        Self::new(inner, RateLimitConfig::default())
    }

    fn refill(&mut self) {
        let now = global_clock().now_ns();
        let elapsed = now.saturating_sub(self.last_refill_ns) as f64 / 1_000_000_000.0;
        if elapsed <= 0.0 {
            return;
        }
        let add = elapsed * self.config.refill_per_sec as f64;
        self.tokens = (self.tokens + add).min(self.config.capacity as f64);
        self.last_refill_ns = now;
    }
}

impl<P, In, Out> Port<In, Out> for RateLimit<P>
where
    P: Port<In, Out>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn call(&mut self, input: In) -> IsaResult<Out> {
        self.name = format!("rate_limit({})", self.inner.name());
        self.refill();
        if self.tokens < 1.0 {
            self.rejected = self.rejected.saturating_add(1);
            return Err(IsaError::port(
                self.name.clone(),
                format!(
                    "rate limited (capacity={}, refill/s={})",
                    self.config.capacity, self.config.refill_per_sec
                ),
            ));
        }
        self.tokens -= 1.0;
        self.inner.call(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_isa::PortFn;

    #[test]
    fn allows_until_capacity_then_rejects() {
        let port = PortFn::new("echo", |x: i32| Ok(x));
        let mut rl = RateLimit::new(
            port,
            RateLimitConfig {
                capacity: 2,
                refill_per_sec: 0, // no refill during test
            },
        );
        assert_eq!(rl.call(1).unwrap(), 1);
        assert_eq!(rl.call(2).unwrap(), 2);
        let err = rl.call(3).unwrap_err().to_string();
        assert!(err.contains("rate limited"), "{err}");
        assert_eq!(rl.rejected, 1);
    }
}
