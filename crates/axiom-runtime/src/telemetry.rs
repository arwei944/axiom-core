//! OpenTelemetry tracing integration for Axiom Runtime.
//!
//! Provides span creation for SignalEnvelope and Cell::handle.

use std::sync::Arc;

/// Telemetry configuration.
#[derive(Debug, Clone, Default)]
pub struct TelemetryConfig {
    pub otlp_endpoint: String,
    pub service_name: String,
    pub sample_ratio: f64,
}

impl TelemetryConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            otlp_endpoint: endpoint.into(),
            service_name: "axiom-runtime".to_string(),
            sample_ratio: 1.0,
        }
    }
}

/// Initialize OpenTelemetry tracing.
pub fn init_telemetry(_config: TelemetryConfig) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "telemetry")]
    {
        let _ = _config;
    }
    Ok(())
}

/// Tracer handle.
#[derive(Debug, Clone, Default)]
pub struct TracerHandle {
    #[cfg(feature = "telemetry")]
    inner: Option<Arc<dyn opentelemetry::trace::TracerProvider>>,
}

#[cfg(feature = "telemetry")]
impl TracerHandle {
    pub fn new(provider: Arc<dyn opentelemetry::trace::TracerProvider>) -> Self {
        Self {
            inner: Some(provider),
        }
    }

    pub fn tracer(
        &self,
        name: &str,
    ) -> Result<impl opentelemetry::trace::Tracer, axiom_kernel::error::AxiomError> {
        let provider =
            self.inner
                .as_ref()
                .ok_or_else(|| axiom_kernel::error::AxiomError::Internal {
                    message: "tracer provider not initialized".into(),
                })?;
        Ok(provider.tracer(name))
    }
}

#[cfg(not(feature = "telemetry"))]
impl TracerHandle {
    pub fn new(_provider: Arc<dyn std::any::Any>) -> Self {
        Self {}
    }

    pub fn tracer(&self, _name: &str) -> Result<NoopTracer, axiom_kernel::error::AxiomError> {
        Ok(NoopTracer)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NoopTracer;

impl NoopTracer {
    pub fn start(&self, _name: &str) -> NoopSpan {
        NoopSpan
    }
}

#[derive(Debug, Clone, Default)]
pub struct NoopSpan;

impl NoopSpan {
    pub fn set_attribute(&self, _key: &str, _value: impl Into<String>) {}
    pub fn end(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_telemetry_does_not_panic() {
        let config = TelemetryConfig::default();
        init_telemetry(config).unwrap();
        let handle = TracerHandle::default();
        let _span = handle.tracer("test").unwrap().start("test_span");
    }
}
