//! Metrics abstraction for observability.
//!
//! Provides a `MetricsRegistry` trait so metrics can be exported in
//! different formats. The default implementation uses `prometheus`.

#[cfg(feature = "metrics")]
use prometheus::{Encoder, HistogramOpts, Opts, Registry, TextEncoder};
#[cfg(feature = "metrics")]
use std::collections::HashMap;

/// Metric type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Metric description.
#[derive(Debug, Clone)]
pub struct MetricDesc {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub labels: Vec<String>,
}

/// Metrics registry abstraction.
pub trait MetricsRegistry: Send + Sync {
    /// Register or get a counter.
    fn register_counter(&mut self, desc: MetricDesc) -> Box<dyn CounterTrait>;

    /// Register or get a gauge.
    fn register_gauge(&mut self, desc: MetricDesc) -> Box<dyn GaugeTrait>;

    /// Register or get a histogram.
    fn register_histogram(&mut self, desc: MetricDesc, buckets: &[f64]) -> Box<dyn HistogramTrait>;

    /// Encode all metrics to a string (e.g., Prometheus text format).
    fn encode(&self) -> String;

    /// Describe all registered metrics.
    fn describe(&self) -> Vec<MetricDesc>;
}

/// Counter trait.
pub trait CounterTrait: Send + Sync {
    fn inc(&self, labels: &[&str]);
    fn inc_by(&self, amount: u64, labels: &[&str]);
}

/// Gauge trait.
pub trait GaugeTrait: Send + Sync {
    fn set(&self, value: f64, labels: &[&str]);
    fn inc(&self, labels: &[&str]);
    fn dec(&self, labels: &[&str]);
}

/// Histogram trait.
pub trait HistogramTrait: Send + Sync {
    fn observe(&self, value: f64, labels: &[&str]);
}

// ---------------------------------------------------------------------------
// No-op implementation (default when `metrics` feature is disabled)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct NoopRegistry;

impl MetricsRegistry for NoopRegistry {
    fn register_counter(&mut self, _desc: MetricDesc) -> Box<dyn CounterTrait> {
        Box::new(NoopCounter)
    }

    fn register_gauge(&mut self, _desc: MetricDesc) -> Box<dyn GaugeTrait> {
        Box::new(NoopGauge)
    }

    fn register_histogram(
        &mut self,
        _desc: MetricDesc,
        _buckets: &[f64],
    ) -> Box<dyn HistogramTrait> {
        Box::new(NoopHistogram)
    }

    fn encode(&self) -> String {
        String::new()
    }

    fn describe(&self) -> Vec<MetricDesc> {
        Vec::new()
    }
}

#[derive(Debug, Default)]
struct NoopCounter;

impl CounterTrait for NoopCounter {
    fn inc(&self, _labels: &[&str]) {}
    fn inc_by(&self, _amount: u64, _labels: &[&str]) {}
}

#[derive(Debug, Default)]
struct NoopGauge;

impl GaugeTrait for NoopGauge {
    fn set(&self, _value: f64, _labels: &[&str]) {}
    fn inc(&self, _labels: &[&str]) {}
    fn dec(&self, _labels: &[&str]) {}
}

#[derive(Debug, Default)]
struct NoopHistogram;

impl HistogramTrait for NoopHistogram {
    fn observe(&self, _value: f64, _labels: &[&str]) {}
}

// ---------------------------------------------------------------------------
// Prometheus implementation (enabled with `metrics` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "metrics")]
#[derive(Debug, Default)]
pub struct PrometheusRegistry {
    registry: Registry,
    counters: HashMap<String, prometheus::Counter>,
    gauges: HashMap<String, prometheus::Gauge>,
    histograms: HashMap<String, prometheus::Histogram>,
}

#[cfg(feature = "metrics")]
impl PrometheusRegistry {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    pub fn get_counter(&self, name: &str) -> Option<&prometheus::Counter> {
        self.counters.get(name)
    }

    pub fn get_gauge(&self, name: &str) -> Option<&prometheus::Gauge> {
        self.gauges.get(name)
    }

    pub fn get_histogram(&self, name: &str) -> Option<&prometheus::Histogram> {
        self.histograms.get(name)
    }
}

#[cfg(feature = "metrics")]
impl MetricsRegistry for PrometheusRegistry {
    fn register_counter(&mut self, desc: MetricDesc) -> Box<dyn CounterTrait> {
        let key = desc.name.clone();
        let opts = Opts::new(desc.name, desc.help);
        let counter = match prometheus::Counter::with_opts(opts) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[axiom-viz] failed to create counter: {}", e);
                return Box::new(NoopCounter);
            }
        };
        self.registry.register(Box::new(counter.clone())).ok();
        self.counters.insert(key, counter.clone());
        Box::new(PrometheusCounter { inner: counter })
    }

    fn register_gauge(&mut self, desc: MetricDesc) -> Box<dyn GaugeTrait> {
        let key = desc.name.clone();
        let opts = Opts::new(desc.name, desc.help);
        let gauge = match prometheus::Gauge::with_opts(opts) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[axiom-viz] failed to create gauge: {}", e);
                return Box::new(NoopGauge);
            }
        };
        self.registry.register(Box::new(gauge.clone())).ok();
        self.gauges.insert(key, gauge.clone());
        Box::new(PrometheusGauge { inner: gauge })
    }

    fn register_histogram(&mut self, desc: MetricDesc, buckets: &[f64]) -> Box<dyn HistogramTrait> {
        let key = desc.name.clone();
        let opts = HistogramOpts::new(desc.name, desc.help).buckets(buckets.to_vec());
        let histogram = match prometheus::Histogram::with_opts(opts) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[axiom-viz] failed to create histogram: {}", e);
                return Box::new(NoopHistogram);
            }
        };
        self.registry.register(Box::new(histogram.clone())).ok();
        self.histograms.insert(key, histogram.clone());
        Box::new(PrometheusHistogram { inner: histogram })
    }

    fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        String::from_utf8(buffer).unwrap_or_default()
    }

    fn describe(&self) -> Vec<MetricDesc> {
        self.registry
            .gather()
            .into_iter()
            .filter_map(|family| {
                let name = family.name().to_string();
                let help = family.help().to_string();
                let metric_type = match family.get_field_type() {
                    prometheus::proto::MetricType::COUNTER => MetricType::Counter,
                    prometheus::proto::MetricType::GAUGE => MetricType::Gauge,
                    prometheus::proto::MetricType::HISTOGRAM => MetricType::Histogram,
                    _ => return None,
                };
                let labels = family.get_metric()[0]
                    .get_label()
                    .iter()
                    .map(|l| l.name().to_string())
                    .collect::<Vec<_>>();
                Some(MetricDesc { name, help, metric_type, labels })
            })
            .collect()
    }
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
struct PrometheusCounter {
    inner: prometheus::Counter,
}

#[cfg(feature = "metrics")]
impl CounterTrait for PrometheusCounter {
    fn inc(&self, _labels: &[&str]) {
        self.inner.inc();
    }

    fn inc_by(&self, amount: u64, _labels: &[&str]) {
        self.inner.inc_by(amount as f64);
    }
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
struct PrometheusGauge {
    inner: prometheus::Gauge,
}

#[cfg(feature = "metrics")]
impl GaugeTrait for PrometheusGauge {
    fn set(&self, value: f64, _labels: &[&str]) {
        self.inner.set(value);
    }

    fn inc(&self, _labels: &[&str]) {
        self.inner.inc();
    }

    fn dec(&self, _labels: &[&str]) {
        self.inner.dec();
    }
}

#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
struct PrometheusHistogram {
    inner: prometheus::Histogram,
}

#[cfg(feature = "metrics")]
impl HistogramTrait for PrometheusHistogram {
    fn observe(&self, value: f64, _labels: &[&str]) {
        self.inner.observe(value);
    }
}

// ---------------------------------------------------------------------------
// Core metric descriptors
// ---------------------------------------------------------------------------

/// Message processing counter.
pub fn message_total() -> MetricDesc {
    MetricDesc {
        name: "axiom_messages_total".to_string(),
        help: "Total number of messages processed by the runtime".to_string(),
        metric_type: MetricType::Counter,
        labels: vec!["layer".to_string(), "signal_type".to_string(), "status".to_string()],
    }
}

/// Message processing duration histogram.
pub fn message_duration_seconds() -> MetricDesc {
    MetricDesc {
        name: "axiom_message_duration_seconds".to_string(),
        help: "Message processing duration in seconds".to_string(),
        metric_type: MetricType::Histogram,
        labels: vec!["layer".to_string(), "cell_id".to_string()],
    }
}

/// Cell restart counter.
pub fn cell_restarts_total() -> MetricDesc {
    MetricDesc {
        name: "axiom_cell_restarts_total".to_string(),
        help: "Total number of cell restarts".to_string(),
        metric_type: MetricType::Counter,
        labels: vec!["cell_id".to_string(), "layer".to_string()],
    }
}

/// Current entropy score gauge.
pub fn entropy_score() -> MetricDesc {
    MetricDesc {
        name: "axiom_entropy_score".to_string(),
        help: "Current entropy score".to_string(),
        metric_type: MetricType::Gauge,
        labels: vec!["cell_id".to_string()],
    }
}

/// Witness chain error counter.
pub fn witness_chain_errors() -> MetricDesc {
    MetricDesc {
        name: "axiom_witness_chain_errors_total".to_string(),
        help: "Total number of witness chain errors".to_string(),
        metric_type: MetricType::Counter,
        labels: vec!["cell_id".to_string()],
    }
}

/// Dead letter queue counter.
pub fn dead_letters_total() -> MetricDesc {
    MetricDesc {
        name: "axiom_dead_letters_total".to_string(),
        help: "Total number of dead letters".to_string(),
        metric_type: MetricType::Counter,
        labels: vec!["signal_type".to_string()],
    }
}

/// Active cells gauge.
pub fn active_cells() -> MetricDesc {
    MetricDesc {
        name: "axiom_active_cells".to_string(),
        help: "Number of currently active cells".to_string(),
        metric_type: MetricType::Gauge,
        labels: vec!["layer".to_string()],
    }
}

/// Initialize core metrics in the given registry.
pub fn init_core_metrics(_registry: &mut dyn MetricsRegistry) {
    #[cfg(feature = "metrics")]
    {
        let _ = _registry;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_registry_does_not_panic() {
        let mut registry = NoopRegistry;
        let counter = registry.register_counter(MetricDesc {
            name: "test_counter".to_string(),
            help: "help".to_string(),
            metric_type: MetricType::Counter,
            labels: Vec::new(),
        });
        counter.inc(&[]);
        counter.inc_by(10, &[]);

        let gauge = registry.register_gauge(MetricDesc {
            name: "test_gauge".to_string(),
            help: "help".to_string(),
            metric_type: MetricType::Gauge,
            labels: Vec::new(),
        });
        gauge.set(1.0, &[]);
        gauge.inc(&[]);
        gauge.dec(&[]);

        let histogram = registry.register_histogram(
            MetricDesc {
                name: "test_histogram".to_string(),
                help: "help".to_string(),
                metric_type: MetricType::Histogram,
                labels: Vec::new(),
            },
            &[0.1, 1.0, 10.0],
        );
        histogram.observe(0.5, &[]);

        assert_eq!(registry.encode(), "");
        assert!(registry.describe().is_empty());
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn prometheus_registry_records_metrics() {
        let mut registry = PrometheusRegistry::new();
        let counter = registry.register_counter(MetricDesc {
            name: "test_counter".to_string(),
            help: "help".to_string(),
            metric_type: MetricType::Counter,
            labels: vec!["label".to_string()],
        });
        counter.inc(&["a"]);
        counter.inc_by(5, &["a"]);

        let gauge = registry.register_gauge(MetricDesc {
            name: "test_gauge".to_string(),
            help: "help".to_string(),
            metric_type: MetricType::Gauge,
            labels: Vec::new(),
        });
        gauge.set(42.0, &[]);

        let encoded = registry.encode();
        assert!(encoded.contains("test_counter"));
        assert!(encoded.contains("test_gauge"));
        assert!(!registry.describe().is_empty());
    }
}
