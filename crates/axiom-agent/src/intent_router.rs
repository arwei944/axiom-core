use std::sync::Arc;

use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRoute {
    pub intent_pattern: String,
    pub target_cell_id: String,
    pub confidence_threshold: f64,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    pub target_cell_id: Option<String>,
    pub matched_intent: String,
    pub confidence: f64,
    pub routing_decision: RoutingDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDecision {
    Routed,
    Direct,
    Rejected,
    Ambiguous,
}

pub struct IntentRouter {
    routes: Arc<RwLock<Vec<IntentRoute>>>,
    fallback_target: String,
    max_confidence_diff: f64,
}

impl IntentRouter {
    pub fn new(fallback_target: &str) -> Self {
        Self {
            routes: Arc::new(RwLock::new(Vec::new())),
            fallback_target: fallback_target.to_string(),
            max_confidence_diff: 0.2,
        }
    }

    pub fn add_route(&self, route: IntentRoute) {
        let mut routes = self.routes.write();
        routes.push(route);
        routes.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn add_routes(&self, routes: Vec<IntentRoute>) {
        let mut all_routes = self.routes.write();
        all_routes.extend(routes);
        all_routes.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn remove_route(&self, intent_pattern: &str) {
        let mut routes = self.routes.write();
        routes.retain(|r| r.intent_pattern != intent_pattern);
    }

    pub fn route(&self, intent: &str, confidence: f64) -> RoutingResult {
        let routes = self.routes.read();
        let mut candidates: Vec<(&IntentRoute, f64)> = Vec::new();

        for route in routes.iter() {
            if self.matches(intent, &route.intent_pattern) {
                let adjusted_confidence = confidence.min(1.0);
                if adjusted_confidence >= route.confidence_threshold {
                    candidates.push((route, adjusted_confidence));
                }
            }
        }

        if candidates.is_empty() {
            return RoutingResult {
                target_cell_id: Some(self.fallback_target.clone()),
                matched_intent: intent.to_string(),
                confidence: 0.0,
                routing_decision: RoutingDecision::Direct,
            };
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best = candidates[0];
        if candidates.len() > 1 {
            let second = candidates[1];
            if (best.1 - second.1).abs() < self.max_confidence_diff {
                return RoutingResult {
                    target_cell_id: None,
                    matched_intent: intent.to_string(),
                    confidence: best.1,
                    routing_decision: RoutingDecision::Ambiguous,
                };
            }
        }

        if best.1 < best.0.confidence_threshold {
            return RoutingResult {
                target_cell_id: Some(self.fallback_target.clone()),
                matched_intent: intent.to_string(),
                confidence: best.1,
                routing_decision: RoutingDecision::Direct,
            };
        }

        RoutingResult {
            target_cell_id: Some(best.0.target_cell_id.clone()),
            matched_intent: intent.to_string(),
            confidence: best.1,
            routing_decision: RoutingDecision::Routed,
        }
    }

    fn matches(&self, intent: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.starts_with("prefix:") {
            let prefix = pattern.strip_prefix("prefix:").unwrap_or("");
            return intent.starts_with(prefix);
        }
        if pattern.starts_with("contains:") {
            let substring = pattern.strip_prefix("contains:").unwrap_or("");
            return intent.contains(substring);
        }
        if pattern.starts_with("regex:") {
            let regex_str = pattern.strip_prefix("regex:").unwrap_or("");
            if let Ok(regex) = Regex::new(regex_str) {
                return regex.is_match(intent);
            }
            return false;
        }
        intent == pattern
    }

    pub fn clear(&self) {
        self.routes.write().clear();
    }

    pub fn route_count(&self) -> usize {
        self.routes.read().len()
    }
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new("agent:default")
    }
}