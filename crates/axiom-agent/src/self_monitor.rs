use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReport {
    pub agent_id: String,
    pub health_status: HealthStatus,
    pub performance_metrics: PerformanceMetrics,
    pub behavior_summary: BehaviorSummary,
    pub confidence_summary: ConfidenceSummary,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_response_time_ms: f64,
    pub error_rate: f64,
    pub throughput: f64,
    pub memory_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSummary {
    pub total_interactions: u64,
    pub successful_interactions: u64,
    pub failed_interactions: u64,
    pub common_intents: Vec<String>,
    pub rare_intents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSummary {
    pub avg_confidence: f64,
    pub low_confidence_count: u64,
    pub high_confidence_count: u64,
    pub confidence_trend: ConfidenceTrend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceTrend {
    Increasing,
    Stable,
    Decreasing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action: String,
    pub reason: String,
    pub priority: u32,
    pub expected_improvement: f64,
}

pub struct SelfMonitor {
    agent_id: String,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    behavior: Arc<RwLock<BehaviorSummary>>,
    confidence: Arc<RwLock<ConfidenceSummary>>,
    confidence_history: Arc<RwLock<Vec<f64>>>,
    sample_count: Arc<RwLock<u64>>,
}

impl SelfMonitor {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                avg_response_time_ms: 0.0,
                error_rate: 0.0,
                throughput: 0.0,
                memory_usage_percent: 0.0,
            })),
            behavior: Arc::new(RwLock::new(BehaviorSummary {
                total_interactions: 0,
                successful_interactions: 0,
                failed_interactions: 0,
                common_intents: Vec::new(),
                rare_intents: Vec::new(),
            })),
            confidence: Arc::new(RwLock::new(ConfidenceSummary {
                avg_confidence: 0.0,
                low_confidence_count: 0,
                high_confidence_count: 0,
                confidence_trend: ConfidenceTrend::Stable,
            })),
            confidence_history: Arc::new(RwLock::new(Vec::new())),
            sample_count: Arc::new(RwLock::new(0)),
        }
    }

    pub fn record_interaction(
        &self,
        success: bool,
        response_time_ms: u64,
        confidence: f64,
        _intent: &str,
    ) {
        let mut metrics = self.metrics.write();
        let mut behavior = self.behavior.write();
        let mut conf = self.confidence.write();
        let mut hist = self.confidence_history.write();
        let mut count = self.sample_count.write();

        behavior.total_interactions += 1;
        if success {
            behavior.successful_interactions += 1;
        } else {
            behavior.failed_interactions += 1;
        }

        hist.push(confidence);
        while hist.len() > 100 {
            hist.remove(0);
        }

        *count += 1;
        let n = *count as f64;
        metrics.avg_response_time_ms =
            ((n - 1.0) * metrics.avg_response_time_ms + response_time_ms as f64) / n;
        metrics.error_rate =
            behavior.failed_interactions as f64 / behavior.total_interactions as f64;

        conf.avg_confidence = ((n - 1.0) * conf.avg_confidence + confidence) / n;
        if confidence < 0.5 {
            conf.low_confidence_count += 1;
        } else if confidence > 0.8 {
            conf.high_confidence_count += 1;
        }

        if hist.len() >= 10 {
            let recent: f64 = hist[(hist.len() - 10)..].iter().sum::<f64>() / 10.0;
            let earlier: f64 = hist[0..10].iter().sum::<f64>() / 10.0;
            conf.confidence_trend = if recent > earlier + 0.1 {
                ConfidenceTrend::Increasing
            } else if recent < earlier - 0.1 {
                ConfidenceTrend::Decreasing
            } else {
                ConfidenceTrend::Stable
            };
        }
    }

    pub fn generate_report(&self) -> SelfReport {
        let metrics = self.metrics.read();
        let behavior = self.behavior.read();
        let conf = self.confidence.read();

        let mut suggestions = Vec::new();

        if metrics.error_rate > 0.2 {
            suggestions.push(SuggestedAction {
                action: "Increase max iterations or timeout".to_string(),
                reason: format!("High error rate: {:.2}%", metrics.error_rate * 100.0),
                priority: 1,
                expected_improvement: 0.3,
            });
        }

        if metrics.avg_response_time_ms > 5000.0 {
            suggestions.push(SuggestedAction {
                action: "Optimize tool execution or reduce memory usage".to_string(),
                reason: format!("Slow response time: {:.0}ms", metrics.avg_response_time_ms),
                priority: 2,
                expected_improvement: 0.25,
            });
        }

        if conf.avg_confidence < 0.5 {
            suggestions.push(SuggestedAction {
                action: "Add more specific instructions or examples".to_string(),
                reason: format!("Low average confidence: {:.2}", conf.avg_confidence),
                priority: 1,
                expected_improvement: 0.35,
            });
        }

        if conf.confidence_trend == ConfidenceTrend::Decreasing {
            suggestions.push(SuggestedAction {
                action: "Review recent changes and adjust strategy".to_string(),
                reason: "Confidence is trending downward".to_string(),
                priority: 2,
                expected_improvement: 0.2,
            });
        }

        let health = if metrics.error_rate > 0.5 || conf.avg_confidence < 0.2 {
            HealthStatus::Critical
        } else if metrics.error_rate > 0.3 || conf.avg_confidence < 0.35 {
            HealthStatus::Unhealthy
        } else if metrics.error_rate > 0.15 || conf.avg_confidence < 0.5 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        SelfReport {
            agent_id: self.agent_id.clone(),
            health_status: health,
            performance_metrics: metrics.clone(),
            behavior_summary: behavior.clone(),
            confidence_summary: conf.clone(),
            suggested_actions: suggestions,
        }
    }

    pub fn reset(&self) {
        *self.metrics.write() = PerformanceMetrics {
            avg_response_time_ms: 0.0,
            error_rate: 0.0,
            throughput: 0.0,
            memory_usage_percent: 0.0,
        };
        *self.behavior.write() = BehaviorSummary {
            total_interactions: 0,
            successful_interactions: 0,
            failed_interactions: 0,
            common_intents: Vec::new(),
            rare_intents: Vec::new(),
        };
        *self.confidence.write() = ConfidenceSummary {
            avg_confidence: 0.0,
            low_confidence_count: 0,
            high_confidence_count: 0,
            confidence_trend: ConfidenceTrend::Stable,
        };
        self.confidence_history.write().clear();
        *self.sample_count.write() = 0;
    }
}
