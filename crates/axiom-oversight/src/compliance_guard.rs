//! ComplianceGuard - PII/sensitive data detection and redaction.

use axiom_kernel::id::CellId;
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceAction {
    Log,
    Warn,
    Redact,
    Reject,
}

#[derive(Debug, Clone)]
pub struct SensitivePattern {
    pub name: &'static str,
    pub regex: &'static str,
    pub severity: Severity,
    pub action: ComplianceAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub pattern: String,
    pub severity: Severity,
    pub match_preview: String,
    pub action_taken: ComplianceAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub violations: Vec<ComplianceViolation>,
    pub redacted_text: Option<String>,
    pub rejected: bool,
}

pub struct ComplianceGuardCell {
    id: CellId,
    patterns: Arc<Mutex<Vec<(SensitivePattern, Regex)>>>,
    violation_counts: Arc<Mutex<HashMap<String, u64>>>,
}

impl ComplianceGuardCell {
    pub fn new() -> Self {
        let builtins = vec![
            SensitivePattern {
                name: "email",
                regex: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
                severity: Severity::Medium,
                action: ComplianceAction::Redact,
            },
            SensitivePattern {
                name: "china_phone",
                regex: r"1[3-9]\d{9}",
                severity: Severity::Medium,
                action: ComplianceAction::Redact,
            },
            SensitivePattern {
                name: "github_token",
                regex: r"ghp_[a-zA-Z0-9]{36,}",
                severity: Severity::Critical,
                action: ComplianceAction::Reject,
            },
            SensitivePattern {
                name: "sk_token",
                regex: r"sk-[a-zA-Z0-9]{20,}",
                severity: Severity::Critical,
                action: ComplianceAction::Reject,
            },
            SensitivePattern {
                name: "bearer_token",
                regex: r"Bearer\s+[a-zA-Z0-9._-]{20,}",
                severity: Severity::Critical,
                action: ComplianceAction::Redact,
            },
            SensitivePattern {
                name: "china_id",
                regex: r"[1-9]\d{5}(18|19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]",
                severity: Severity::Critical,
                action: ComplianceAction::Redact,
            },
        ];
        let compiled: Vec<(SensitivePattern, Regex)> = builtins
            .into_iter()
            .filter_map(|p| Regex::new(p.regex).ok().map(|r| (p.clone(), r)))
            .collect();
        Self {
            id: CellId::new("oversight:compliance-guard"),
            patterns: Arc::new(Mutex::new(compiled)),
            violation_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn id(&self) -> &CellId {
        &self.id
    }

    pub fn add_pattern(&self, pattern: SensitivePattern) -> Result<(), regex::Error> {
        let re = Regex::new(pattern.regex)?;
        self.patterns.lock().push((pattern, re));
        Ok(())
    }

    pub fn check_text(&self, text: &str) -> ComplianceResult {
        let mut violations = Vec::new();
        let mut redacted = text.to_string();
        let mut rejected = false;

        for (pat, re) in self.patterns.lock().iter() {
            for m in re.find_iter(text) {
                *self.violation_counts.lock().entry(pat.name.to_string()).or_insert(0) += 1;

                let preview: String = m.as_str().chars().take(8).collect();
                violations.push(ComplianceViolation {
                    pattern: pat.name.to_string(),
                    severity: pat.severity,
                    match_preview: preview,
                    action_taken: pat.action,
                });

                match pat.action {
                    ComplianceAction::Reject => rejected = true,
                    ComplianceAction::Redact => {
                        let replace = "[REDACTED_".to_string() + pat.name + "]";
                        redacted = re.replace_all(&redacted, replace.as_str()).to_string();
                    }
                    ComplianceAction::Log | ComplianceAction::Warn => {}
                }
            }
        }

        ComplianceResult {
            violations,
            redacted_text: if redacted != text { Some(redacted) } else { None },
            rejected,
        }
    }

    pub fn check_json(&self, value: &serde_json::Value) -> ComplianceResult {
        let text = serde_json::to_string(value).unwrap_or_default();
        self.check_text(&text)
    }

    pub fn stats(&self) -> HashMap<String, u64> {
        self.violation_counts.lock().clone()
    }
}

impl Default for ComplianceGuardCell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_email_and_redacts() {
        let c = ComplianceGuardCell::new();
        let addr = String::from("f") + "oo@example.com";
        let text = format!("contact me at {} please", addr);
        let r = c.check_text(&text);
        assert!(
            r.violations.iter().any(|v| v.pattern == "email"),
            "email pattern should match; violations {:?}",
            r.violations
        );
        assert!(r.redacted_text.is_some());
        assert!(!r.rejected);
        assert!(!r.redacted_text.unwrap().contains("oo@"));
    }

    #[test]
    fn test_detects_github_token_rejected() {
        let c = ComplianceGuardCell::new();
        let fake = "ghp_".to_string() + &"a".repeat(40);
        let r = c.check_text(&format!("token is {}", fake));
        assert!(r.rejected);
        assert!(r.violations.iter().any(|v| v.severity == Severity::Critical));
    }

    #[test]
    fn test_clean_text_no_violations() {
        let c = ComplianceGuardCell::new();
        let r = c.check_text("hello world, this is normal text 12345");
        assert!(r.violations.is_empty());
        assert!(!r.rejected);
        assert!(r.redacted_text.is_none());
    }

    #[test]
    fn test_detects_china_phone() {
        let c = ComplianceGuardCell::new();
        let r = c.check_text("call me at 13812345678");
        assert!(r.violations.iter().any(|v| v.pattern == "china_phone"));
    }
}
