use axiom_isa::{IsaError, IsaResult, Port};
use axiom_kernel::clock::global_clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitConfig {
    pub failure_threshold: u32,
    pub reset_after_ms: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            reset_after_ms: 1_000,
        }
    }
}

/// Circuit breaker around a Port.
pub struct CircuitBreaker<P> {
    name: String,
    inner: P,
    config: CircuitConfig,
    state: CircuitState,
    consecutive_failures: u32,
    opened_at_ns: Option<u64>,
    /// How many times call short-circuited while open.
    pub rejected_while_open: u32,
}

impl<P> CircuitBreaker<P> {
    pub fn new(inner: P, config: CircuitConfig) -> Self {
        Self {
            name: "circuit".into(),
            inner,
            config,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_ns: None,
            rejected_while_open: 0,
        }
    }

    pub fn wrap(inner: P) -> Self {
        Self::new(inner, CircuitConfig::default())
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn into_inner(self) -> P {
        self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    pub fn force_open(&mut self) {
        self.state = CircuitState::Open;
        self.opened_at_ns = Some(global_clock().now_ns());
    }

    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.consecutive_failures = 0;
        self.opened_at_ns = None;
    }

    fn maybe_half_open(&mut self) {
        if self.state != CircuitState::Open {
            return;
        }
        let opened = match self.opened_at_ns {
            Some(t) => t,
            None => return,
        };
        let now = global_clock().now_ns();
        let elapsed_ms = now.saturating_sub(opened) / 1_000_000;
        if elapsed_ms >= self.config.reset_after_ms {
            self.state = CircuitState::HalfOpen;
        }
    }
}

impl<P, In, Out> Port<In, Out> for CircuitBreaker<P>
where
    P: Port<In, Out>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn call(&mut self, input: In) -> IsaResult<Out> {
        self.name = format!("circuit({})", self.inner.name());
        self.maybe_half_open();

        if self.state == CircuitState::Open {
            self.rejected_while_open = self.rejected_while_open.saturating_add(1);
            return Err(IsaError::CircuitOpen {
                name: self.name.clone(),
            });
        }

        match self.inner.call(input) {
            Ok(out) => {
                self.consecutive_failures = 0;
                self.state = CircuitState::Closed;
                self.opened_at_ns = None;
                Ok(out)
            }
            Err(e) => {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                if self.consecutive_failures >= self.config.failure_threshold
                    || self.state == CircuitState::HalfOpen
                {
                    self.state = CircuitState::Open;
                    self.opened_at_ns = Some(global_clock().now_ns());
                }
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_isa::PortFn;

    #[test]
    fn opens_after_threshold() {
        let port = PortFn::new("bad", |_x: ()| -> IsaResult<()> {
            Err(IsaError::port("bad", "fail"))
        });
        let mut cb = CircuitBreaker::new(
            port,
            CircuitConfig {
                failure_threshold: 2,
                reset_after_ms: 60_000,
            },
        );
        assert!(cb.call(()).is_err());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.call(()).is_err());
        assert_eq!(cb.state(), CircuitState::Open);
        // third call short-circuits
        let err = cb.call(()).unwrap_err();
        assert!(matches!(err, IsaError::CircuitOpen { .. }));
        assert_eq!(cb.rejected_while_open, 1);
    }

    #[test]
    fn success_resets_failures() {
        let mut n = 0;
        let port = PortFn::new("ok", move |_x: ()| {
            n += 1;
            if n == 1 {
                Err(IsaError::port("ok", "once"))
            } else {
                Ok(1)
            }
        });
        let mut cb = CircuitBreaker::new(
            port,
            CircuitConfig {
                failure_threshold: 3,
                reset_after_ms: 60_000,
            },
        );
        assert!(cb.call(()).is_err());
        assert_eq!(cb.call(()).unwrap(), 1);
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
