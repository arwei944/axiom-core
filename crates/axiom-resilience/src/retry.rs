use axiom_isa::{IsaError, IsaResult, Port};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

/// Retries a failing Port up to `max_attempts` times.
pub struct Retry<P> {
    name: String,
    inner: P,
    config: RetryConfig,
    /// Total attempts observed (success + fail), for tests/demo.
    pub attempts: u32,
}

impl<P> Retry<P> {
    pub fn new(inner: P, config: RetryConfig) -> Self {
        let name = format!("retry({})", "port");
        Self {
            name,
            inner,
            config,
            attempts: 0,
        }
    }

    pub fn wrap(inner: P) -> Self {
        Self::new(inner, RetryConfig::default())
    }

    pub fn into_inner(self) -> P {
        self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<P, In, Out> Port<In, Out> for Retry<P>
where
    P: Port<In, Out>,
    In: Clone,
{
    fn name(&self) -> &str {
        // Prefer outer name including inner for journal clarity
        &self.name
    }

    fn call(&mut self, input: In) -> IsaResult<Out> {
        // Refresh display name once we can see inner
        self.name = format!("retry({})", self.inner.name());
        let mut last_err = IsaError::port(self.inner.name(), "no attempts");
        for attempt in 1..=self.config.max_attempts {
            self.attempts = self.attempts.saturating_add(1);
            match self.inner.call(input.clone()) {
                Ok(out) => return Ok(out),
                Err(e) => {
                    last_err = e;
                    if attempt == self.config.max_attempts {
                        break;
                    }
                }
            }
        }
        Err(IsaError::port(
            self.name.clone(),
            format!(
                "exhausted {} attempts: {last_err}",
                self.config.max_attempts
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_isa::PortFn;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    #[test]
    fn retries_until_success() {
        let hits = Arc::new(AtomicU32::new(0));
        let h = hits.clone();
        let port = PortFn::new("flaky", move |_x: i32| {
            let n = h.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                Err(IsaError::port("flaky", "not yet"))
            } else {
                Ok(n)
            }
        });
        let mut r = Retry::new(port, RetryConfig { max_attempts: 5 });
        assert_eq!(r.call(1).unwrap(), 3);
        assert_eq!(r.attempts, 3);
    }

    #[test]
    fn exhausts_attempts() {
        let port = PortFn::new("always_bad", |_x: i32| -> IsaResult<i32> {
            Err(IsaError::port("always_bad", "nope"))
        });
        let mut r = Retry::new(port, RetryConfig { max_attempts: 2 });
        assert!(r.call(0).is_err());
        assert_eq!(r.attempts, 2);
    }
}
