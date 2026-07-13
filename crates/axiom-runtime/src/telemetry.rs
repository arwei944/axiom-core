use std::sync::Arc;

#[cfg(feature = "telemetry")]
use opentelemetry::trace::TracerProvider;

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

pub fn init_telemetry(_config: TelemetryConfig) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "telemetry")]
    {
        let _ = _config;
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct TracerHandle {
    #[cfg(feature = "telemetry")]
    inner: Option<Arc<opentelemetry_sdk::trace::SdkTracerProvider>>,
}

#[cfg(feature = "telemetry")]
impl TracerHandle {
    pub fn new(provider: Arc<opentelemetry_sdk::trace::SdkTracerProvider>) -> Self {
        Self { inner: Some(provider) }
    }

    pub fn tracer(
        &self,
        name: &'static str,
    ) -> Result<opentelemetry_sdk::trace::Tracer, axiom_kernel::error::AxiomError> {
        let provider = self.inner.as_ref().ok_or_else(|| {
            axiom_kernel::error::AxiomError::InternalError("tracer provider not initialized".into())
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

    #[cfg(feature = "telemetry")]
    use opentelemetry::trace::{Span, Tracer};

    #[test]
    fn noop_telemetry_does_not_panic() {
        let config = TelemetryConfig::default();
        init_telemetry(config).unwrap();
        let handle = TracerHandle::default();
        let _ = handle.tracer("test");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn telemetry_with_provider() {
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
        let handle = TracerHandle::new(Arc::new(provider));
        let mut span = handle.tracer("test").unwrap().start("test_span");
        span.end();
    }
}
