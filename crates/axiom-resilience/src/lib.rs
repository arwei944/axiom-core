//! U2 resilience primitives wrapping ISA [`Port`]s.
//!
//! Constitution: resilience is standard library decoration — not a second runtime.

mod bulkhead;
mod circuit;
mod rate_limit;
mod retry;

pub use bulkhead::{Bulkhead, BulkheadConfig};
pub use circuit::{CircuitBreaker, CircuitConfig, CircuitState};
pub use rate_limit::{RateLimit, RateLimitConfig};
pub use retry::{Retry, RetryConfig};
