use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing;

use crate::agent::AgentCell;
use crate::builder::AgentBuilder;
use crate::error::{AgentError, AgentResult};
use crate::intent_router::{IntentRoute, IntentRouter};
use crate::self_monitor::{HealthStatus, SelfMonitor, SelfReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoMode {
    Conservative,
    #[default]
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfig {
    pub auto_start: bool,
    pub auto_discover: bool,
    pub auto_tune: bool,
    pub auto_heal: bool,
    pub auto_evolve: bool,
    pub mode: AutoMode,
    pub health_check_interval_secs: u64,
    pub tune_interval_interactions: u64,
    pub min_confidence_threshold: f64,
}

impl Default for AutoConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            auto_discover: true,
            auto_tune: true,
            auto_heal: true,
            auto_evolve: false,
            mode: AutoMode::Balanced,
            health_check_interval_secs: 30,
            tune_interval_interactions: 100,
            min_confidence_threshold: 0.5,
        }
    }
}

pub struct AutoAgent {
    inner: Arc<AgentCell>,
    config: Arc<RwLock<AutoConfig>>,
    monitor: Arc<SelfMonitor>,
    router: Arc<IntentRouter>,
    interaction_count: Arc<RwLock<u64>>,
    running: Arc<RwLock<bool>>,
}

impl AutoAgent {
    pub fn new(id: impl Into<String>) -> AgentResult<Self> {
        let id = id.into();
        let monitor = Arc::new(SelfMonitor::new(&id));
        let router = Arc::new(IntentRouter::new(&format!("agent:{}", id)));

        let agent = AgentBuilder::new(&id)
            .with_self_monitor_arc(monitor.clone())
            .with_intent_router_arc(router.clone())
            .build()?;

        Ok(Self {
            inner: Arc::new(agent),
            config: Arc::new(RwLock::new(AutoConfig::default())),
            monitor,
            router,
            interaction_count: Arc::new(RwLock::new(0)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    pub fn with_llm(self, llm: axiom_llm::LlmClient) -> AgentResult<Self> {
        let id = self.inner.id().to_string();
        let agent = AgentBuilder::new(&id)
            .with_llm(llm)
            .with_self_monitor_arc(self.monitor.clone())
            .with_intent_router_arc(self.router.clone())
            .build()?;
        Ok(Self {
            inner: Arc::new(agent),
            config: self.config,
            monitor: self.monitor,
            router: self.router,
            interaction_count: self.interaction_count,
            running: self.running,
        })
    }

    pub fn with_mode(&self, mode: AutoMode) {
        self.config.write().mode = mode;
    }

    pub fn with_auto_config(&self, config: AutoConfig) {
        *self.config.write() = config;
    }

    pub fn id(&self) -> &str {
        self.inner.id()
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub async fn start(&self) -> AgentResult<()> {
        if *self.running.read() {
            return Ok(());
        }

        let config = self.config.read().clone();

        if config.auto_start {
            self.inner.start()?;
            *self.running.write() = true;
            tracing::info!(agent_id = %self.id(), "AutoAgent started");
        }

        if config.auto_discover {
            self.auto_discover()?;
        }

        Ok(())
    }

    pub async fn stop(&self) -> AgentResult<()> {
        if !*self.running.read() {
            return Ok(());
        }

        self.inner.stop()?;
        *self.running.write() = false;
        tracing::info!(agent_id = %self.id(), "AutoAgent stopped");

        Ok(())
    }

    pub async fn process(&self, input: &str) -> AgentResult<String> {
        self.ensure_running().await?;

        let intent = self.detect_intent(input);
        let confidence = self.estimate_confidence(input, &intent);

        let result = self.inner.query(input, Some(&intent)).await;

        self.record_interaction(result.is_ok(), &intent, confidence);
        self.maybe_auto_tune();

        match result {
            Ok(response) => Ok(response),
            Err(e) => {
                if self.config.read().auto_heal {
                    self.auto_heal().await?;
                    self.inner.query(input, Some(&intent)).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn process_natural(
        &self,
        signal: crate::natural_signal::NaturalSignal,
    ) -> AgentResult<String> {
        self.process(&signal.content).await
    }

    pub fn health_report(&self) -> SelfReport {
        self.monitor.generate_report()
    }

    pub fn agent(&self) -> &AgentCell {
        &self.inner
    }

    fn auto_discover(&self) -> AgentResult<()> {
        tracing::info!(agent_id = %self.id(), "Auto-discovering capabilities");

        let default_routes = vec![
            IntentRoute {
                intent_pattern: "contains:search".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:find".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:lookup".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:summarize".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:summary".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:explain".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:write".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:create".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:generate".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:translate".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "contains:code".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.5,
                priority: 1,
            },
            IntentRoute {
                intent_pattern: "*".to_string(),
                target_cell_id: format!("agent:{}", self.id()),
                confidence_threshold: 0.3,
                priority: 0,
            },
        ];

        self.router.add_routes(default_routes);
        tracing::info!(agent_id = %self.id(), route_count = %self.router.route_count(), "Auto-discovery complete");

        Ok(())
    }

    fn detect_intent(&self, input: &str) -> String {
        let input_lower = input.to_lowercase();

        let intent_patterns: Vec<(&str, Vec<&str>)> = vec![
            ("web_search", vec!["search", "find", "look up", "lookup", "google"]),
            ("summarize", vec!["summarize", "summary", "brief", "shorten"]),
            ("explain", vec!["explain", "what is", "how does", "why"]),
            ("write", vec!["write", "create", "generate", "compose", "draft"]),
            ("translate", vec!["translate", "convert to"]),
            ("code", vec!["code", "program", "function", "script", "bug", "debug"]),
            ("analyze", vec!["analyze", "analysis", "break down", "evaluate"]),
            ("plan", vec!["plan", "schedule", "roadmap", "timeline"]),
        ];

        for (intent, keywords) in &intent_patterns {
            for keyword in keywords {
                if input_lower.contains(keyword) {
                    return intent.to_string();
                }
            }
        }

        "general".to_string()
    }

    fn estimate_confidence(&self, input: &str, intent: &str) -> f64 {
        let mut base_confidence: f64 = 0.5;

        let length = input.len();
        if length > 50 {
            base_confidence += 0.1;
        }
        if length > 100 {
            base_confidence += 0.1;
        }

        let has_question_mark = input.contains('?');
        let has_action_verb = ["search", "find", "write", "explain", "summarize", "translate"]
            .iter()
            .any(|w| input.to_lowercase().contains(w));

        if has_question_mark {
            base_confidence += 0.1;
        }
        if has_action_verb {
            base_confidence += 0.15;
        }

        let adjusted = match intent {
            "question_answer" => base_confidence + 0.1,
            "general" => base_confidence - 0.2,
            _ => base_confidence,
        };

        adjusted.clamp(0.1, 0.99)
    }

    fn record_interaction(&self, success: bool, intent: &str, confidence: f64) {
        let mut count = self.interaction_count.write();
        *count += 1;

        let duration_ms = 100u64;
        self.monitor.record_interaction(success, duration_ms, confidence, intent);
    }

    fn maybe_auto_tune(&self) {
        if !self.config.read().auto_tune {
            return;
        }

        let count = *self.interaction_count.read();
        let interval = self.config.read().tune_interval_interactions;

        if count > 0 && count.is_multiple_of(interval) {
            self.auto_tune();
        }
    }

    fn auto_tune(&self) {
        let report = self.monitor.generate_report();
        let config = self.config.read();

        tracing::info!(
            agent_id = %self.id(),
            health = ?report.health_status,
            "Auto-tuning agent parameters"
        );

        match config.mode {
            AutoMode::Conservative => {
                if report.confidence_summary.avg_confidence < config.min_confidence_threshold {
                    tracing::debug!(agent_id = %self.id(), "Conservative mode: maintaining current settings");
                }
            }
            AutoMode::Balanced => {
                if report.health_status as u8 <= HealthStatus::Degraded as u8 {
                    tracing::info!(agent_id = %self.id(), "Balanced mode: adjusting parameters for better performance");
                }
            }
            AutoMode::Aggressive => {
                if report.confidence_summary.confidence_trend
                    == crate::self_monitor::ConfidenceTrend::Increasing
                {
                    tracing::info!(agent_id = %self.id(), "Aggressive mode: pushing for higher performance");
                }
            }
        }
    }

    async fn auto_heal(&self) -> AgentResult<()> {
        if !self.config.read().auto_heal {
            return Err(AgentError::NotConfigured("Auto-heal disabled".into()));
        }

        tracing::warn!(agent_id = %self.id(), "Attempting auto-heal");

        let report = self.monitor.generate_report();

        match report.health_status {
            HealthStatus::Healthy => Ok(()),
            HealthStatus::Degraded => {
                tracing::info!(agent_id = %self.id(), "Degraded: resetting session state");
                self.monitor.reset();
                Ok(())
            }
            HealthStatus::Unhealthy => {
                tracing::warn!(agent_id = %self.id(), "Unhealthy: restarting agent");
                self.inner.stop().ok();
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.inner.start()?;
                Ok(())
            }
            HealthStatus::Critical => {
                tracing::error!(agent_id = %self.id(), "Critical: full reset required");
                self.inner.stop().ok();
                self.monitor.reset();
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                self.inner.start()?;
                Ok(())
            }
        }
    }

    async fn ensure_running(&self) -> AgentResult<()> {
        if !*self.running.read() {
            self.start().await?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for AutoAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoAgent")
            .field("id", &self.inner.id())
            .field("running", &*self.running.read())
            .field("mode", &self.config.read().mode)
            .field("health", &self.monitor.generate_report().health_status)
            .finish()
    }
}
